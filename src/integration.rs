//! Keep AdGuard's app-scoped SOCKS route in step with explicit VPN actions.
//! Status checks never clear an enabled route after an unexpected VPN failure.
use crate::{cli, model, preferences::Preferences};
use gtk::gio::{self, prelude::*};
use std::{env, path::PathBuf};

const CONTROL_TIMEOUT: u32 = 8;

struct Outbound {
    enabled: bool,
    compatible: bool,
}

fn value(text: &str, key: &str) -> Option<String> {
    model::config_value(text, key)
        .map(|s| s.trim_matches(['\'', '"']).to_owned())
}

async fn adguard(prefs: &Preferences, args: &[&str]) -> Result<String, String> {
    cli::run(
        &prefs.adguard_binary_path,
        &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        CONTROL_TIMEOUT,
    )
    .await
    .map_err(|e| format!("광고 차단 연동 실패: {e}"))
}

async fn outbound(prefs: &Preferences) -> Result<Outbound, String> {
    let text = adguard(prefs, &["config", "show", "outbound_proxy"]).await?;
    let enabled = match value(&text, "enabled").as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => return Err("광고 차단의 프록시 전달 상태를 읽지 못했습니다.".into()),
    };
    Ok(Outbound {
        enabled,
        compatible: value(&text, "mode").is_some_and(|s| s.eq_ignore_ascii_case("SOCKS5"))
            && value(&text, "host").as_deref() == Some("127.0.0.1")
            && value(&text, "port").as_deref() == Some("1080"),
    })
}

async fn set_route(prefs: &Preferences, enabled: bool) -> Result<(), String> {
    let output = adguard(
        prefs,
        &[
            "config",
            "set",
            "outbound_proxy.enabled",
            if enabled { "true" } else { "false" },
        ],
    )
    .await?;
    let clean = model::clean(&output);
    if !clean.lines().any(|line| line.trim() == "Config has been updated") {
        return Err(format!("프록시 전달 설정을 적용하지 못했습니다: {clean}"));
    }
    if outbound(prefs).await?.enabled != enabled {
        return Err("프록시 전달 설정이 저장되지 않았습니다. 연결 상태를 확인하세요.".into());
    }
    let lower = clean.to_lowercase();
    if lower.contains("restart the proxy server")
        || lower.contains("failed to apply settings to running proxy server")
    {
        cli::run(&prefs.adguard_binary_path, &["restart".into()], 20).await?;
    }
    if enabled {
        require_filtering(prefs).await?;
    }
    Ok(())
}

async fn require_filtering(prefs: &Preferences) -> Result<(), String> {
    let text = adguard(prefs, &["status"]).await?.to_lowercase();
    if !model::clean(&text).contains("proxy server is running")
        || !model::clean(&text).contains("system-wide automatic filtering is enabled")
    {
        return Err("자동 광고 차단이 꺼져 있어 대상 앱을 VPN으로 전달할 수 없습니다. 광고 차단을 먼저 켜세요.".into());
    }
    Ok(())
}

async fn require_socks(prefs: &Preferences) -> Result<(), String> {
    if !targets_path().is_file() {
        return Err("VPN 대상 목록 파일을 찾지 못했습니다. 호환 설정을 복구한 뒤 연결하세요.".into());
    }
    let text = cli::run(
        &prefs.binary_path,
        &["config".into(), "show".into()],
        CONTROL_TIMEOUT,
    )
    .await?;
    if !value(&text, "Mode").is_some_and(|s| s.eq_ignore_ascii_case("socks"))
        || value(&text, "SOCKS host").as_deref() != Some("127.0.0.1")
        || value(&text, "SOCKS port").as_deref() != Some("1080")
    {
        return Err("이 연동은 SOCKS / 127.0.0.1:1080 설정이 필요합니다. TUN 모드로 연결하지 않았습니다.".into());
    }
    if !outbound(prefs).await?.compatible {
        return Err("광고 차단의 전달 주소가 VPN과 다릅니다. SOCKS5 / 127.0.0.1:1080 설정을 확인하세요.".into());
    }
    require_filtering(prefs).await
}

fn attention(reason: &str) -> String {
    format!("ADGUARD_ROUTE_ATTENTION: {reason}")
}

pub async fn status(prefs: &Preferences) -> Result<String, String> {
    let text = cli::run(&prefs.binary_path, &["status".into()], CONTROL_TIMEOUT).await?;
    let route = outbound(prefs).await?;
    if model::parse_status(&text).connected() {
        if !text.to_lowercase().contains("in socks mode") || !route.compatible {
            return Ok(attention("호환 모드·전달 주소 확인 필요"));
        }
        if !route.enabled {
            return Ok(attention("VPN만 연결됨 · 앱 전달 꺼짐"));
        }
        if let Err(e) = require_filtering(prefs).await {
            return Ok(attention(&e));
        }
    } else if route.enabled {
        return Ok(attention("VPN 연결 확인 불가 · 전달 설정 유지. 일반 연결은 연결 해제로 전환하세요."));
    }
    Ok(text)
}

pub async fn connect(prefs: &Preferences, args: &[String]) -> Result<String, String> {
    require_socks(prefs).await?;
    cli::run(&prefs.binary_path, args, 30).await?;
    let text = cli::run(&prefs.binary_path, &["status".into()], CONTROL_TIMEOUT).await?;
    if !model::parse_status(&text).connected()
        || !text.to_lowercase().contains("in socks mode")
    {
        return Err("VPN의 SOCKS 연결 완료를 확인하지 못했습니다. 앱 전달 상태는 임의로 해제하지 않았습니다.".into());
    }
    let client = gio::SocketClient::new();
    client.set_enable_proxy(false);
    client.set_timeout(3);
    let connection = client
        .connect_to_host_future("127.0.0.1", 1080)
        .await
        .map_err(|e| format!("로컬 VPN 프록시가 준비되지 않았습니다: {e}"))?;
    drop(connection);
    require_filtering(prefs).await?;
    set_route(prefs, true).await?;
    Ok("대상 앱의 VPN 전달을 켰습니다.".into())
}

pub async fn disconnect(prefs: &Preferences) -> Result<String, String> {
    // Only an explicit Disconnect action restores direct routing.
    // If changing AdGuard fails, do not tear down the VPN underneath it.
    set_route(prefs, false).await?;
    cli::run(&prefs.binary_path, &["disconnect".into()], 20)
        .await
        .map_err(|e| format!("대상 앱은 일반 연결로 전환했지만 VPN 종료를 확인하지 못했습니다: {e}"))
}

pub fn targets_path() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
        .join("adguard-cli/vpn-apps.yaml")
}
