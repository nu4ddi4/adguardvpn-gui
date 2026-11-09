//! 고정된 프로젝트 릴리스에서 RPM을 받아 무결성을 확인한 뒤 시스템 인증으로 설치합니다.
use crate::cli;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const REPOSITORY: &str = "nu4ddi4/adguardvpn-gui";
#[derive(Clone, Debug)]
pub struct Package {
    pub version: String,
    pub path: PathBuf,
    pub sha256: String,
}

pub fn version(value: &str) -> Option<(u32, u32, u32)> {
    let numbers: Vec<_> = value
        .strip_prefix('v')
        .unwrap_or(value)
        .split('.')
        .collect();
    if numbers.len() != 3
        || numbers
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some((
        numbers[0].parse().ok()?,
        numbers[1].parse().ok()?,
        numbers[2].parse().ok()?,
    ))
}

fn asset(release: &Value, name: &str, tag: &str) -> Result<String, String> {
    let expected = format!("https://github.com/{REPOSITORY}/releases/download/{tag}/{name}");
    release["assets"]
        .as_array()
        .and_then(|assets| assets.iter().find(|a| a["name"].as_str() == Some(name)))
        .and_then(|a| a["browser_download_url"].as_str())
        .filter(|url| *url == expected)
        .map(str::to_owned)
        .ok_or_else(|| format!("공식 프로젝트 릴리스의 {name} 파일을 확인하지 못했습니다."))
}

pub fn checksum(manifest: &str, name: &str) -> Result<String, String> {
    let values: Vec<_> = manifest
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() == 2 && fields[1].trim_start_matches('*') == name {
                Some(fields[0])
            } else {
                None
            }
        })
        .collect();
    if values.len() != 1
        || values[0].len() != 64
        || !values[0].bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("RPM 체크섬이 없거나 형식이 잘못되었습니다.".into());
    }
    Ok(values[0].to_ascii_lowercase())
}

pub fn verify(path: &Path, expected: &str) -> Result<(), String> {
    let mut input = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    if format!("{:x}", hasher.finalize()) != expected {
        return Err("RPM 체크섬이 일치하지 않아 설치하지 않았습니다.".into());
    }
    Ok(())
}

fn curl_args(url: &str, limit: u64) -> Vec<String> {
    [
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--connect-timeout",
        "15",
        "--max-time",
        "120",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--user-agent",
        "adguard-gui-updater",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(["--max-filesize".into(), limit.to_string(), url.into()])
    .collect()
}

pub async fn prepare(current: &str) -> Result<Option<Package>, String> {
    let json = cli::run(
        "curl",
        &curl_args(
            &format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=20"),
            8 * 1024 * 1024,
        ),
        130,
    )
    .await?;
    let releases: Vec<Value> =
        serde_json::from_str(&json).map_err(|e| format!("릴리스 응답 오류: {e}"))?;
    let current = version(current).ok_or("현재 앱 버전을 읽지 못했습니다.")?;
    let latest = releases
        .iter()
        .filter(|r| r["draft"] != true)
        .filter_map(|r| Some((version(r["tag_name"].as_str()?)?, r)))
        .filter(|(v, _)| *v > current)
        .max_by_key(|(v, _)| *v);
    let Some((_, release)) = latest else {
        return Ok(None);
    };
    let tag = release["tag_name"]
        .as_str()
        .ok_or("릴리스 태그가 없습니다.")?;
    let arch = std::env::consts::ARCH;
    if !["x86_64", "aarch64"].contains(&arch) {
        return Err("이 CPU의 자동 업데이트 RPM이 없습니다.".into());
    }
    let name = format!("adguard-gui-fedora.{arch}.rpm");
    let rpm_url = asset(release, &name, tag)?;
    let hash_url = asset(release, "SHA256SUMS", tag)?;
    let hashes = cli::run("curl", &curl_args(&hash_url, 1024 * 1024), 130).await?;
    let sha256 = checksum(&hashes, &name)?;
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
        });
    let root = base.join("adguard-gui/updates");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&root)
        .map_err(|e| e.to_string())?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = root.join(format!("{tag}-{}-{unique}", std::process::id()));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .map_err(|e| e.to_string())?;
    let temporary = directory.join("package.part");
    let path = directory.join(name);
    let mut args = curl_args(&rpm_url, 100 * 1024 * 1024);
    args.extend(["--output".into(), temporary.to_string_lossy().into_owned()]);
    if let Err(error) = cli::run("curl", &args, 130).await {
        let _ = fs::remove_dir_all(&directory);
        return Err(error);
    }
    if let Err(error) = verify(&temporary, &sha256) {
        let _ = fs::remove_dir_all(&directory);
        return Err(error);
    }
    fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
    Ok(Some(Package {
        version: tag.into(),
        path,
        sha256,
    }))
}

pub async fn install(package: &Package) -> Result<(), String> {
    verify(&package.path, &package.sha256)?;
    let manifest = fs::read_to_string("/etc/os-release").map_err(|e| e.to_string())?;
    if !manifest
        .lines()
        .any(|line| line == "ID=fedora" || line == "ID=\"fedora\"")
        || Path::new("/run/ostree-booted").exists()
    {
        return Err(
            "자동 설치는 일반 Fedora 데스크톱에서 지원합니다. 릴리스 RPM으로 직접 업데이트하세요."
                .into(),
        );
    }
    if !Path::new("/usr/bin/pkexec").exists() || !Path::new("/usr/bin/dnf").exists() {
        return Err("시스템 인증 도구가 없습니다. 내려받은 RPM을 DNF로 설치하세요.".into());
    }
    cli::run(
        "/usr/bin/pkexec",
        &[
            "/usr/bin/dnf".into(),
            "-y".into(),
            "install".into(),
            package.path.to_string_lossy().into_owned(),
        ],
        600,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_are_numeric_and_strict() {
        assert!(version("v0.10.0").unwrap() > version("0.9.0").unwrap());
        for value in ["v1.2", "v1.2.3/../../x", "1.2.3-beta", "999999999999.1.1"] {
            assert!(version(value).is_none());
        }
    }
    #[test]
    fn reject_foreign_assets_and_duplicate_hashes() {
        let release = serde_json::json!({"assets":[{"name":"a.rpm","browser_download_url":"https://evil.example/a.rpm"}]});
        assert!(asset(&release, "a.rpm", "v1.0.0").is_err());
        let line = format!("{}  a.rpm\n", "a".repeat(64));
        assert!(checksum(&line, "a.rpm").is_ok());
        assert!(checksum(&(line.clone() + &line), "a.rpm").is_err());
    }
    #[test]
    fn checksum_rejects_modified_packages() {
        let path = std::env::temp_dir().join(format!("adguard-checksum-{}", std::process::id()));
        fs::write(&path, b"abc").unwrap();
        assert!(verify(
            &path,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )
        .is_ok());
        fs::write(&path, b"modified").unwrap();
        assert!(verify(&path, &"a".repeat(64)).is_err());
        let _ = fs::remove_file(path);
    }
    #[test]
    #[ignore = "공개 GitHub 릴리스 다운로드를 별도로 검증할 때 실행"]
    fn public_release_download() {
        let cache =
            std::env::temp_dir().join(format!("adguard-update-network-{}", std::process::id()));
        std::env::set_var("XDG_CACHE_HOME", &cache);
        let context = gtk::glib::MainContext::new();
        let package = context
            .with_thread_default(|| context.block_on(prepare("0.0.0")))
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(package.path.is_file());
        verify(&package.path, &package.sha256).unwrap();
        println!("공개 릴리스 {} 다운로드·SHA-256 검증 성공", package.version);
        fs::remove_dir_all(cache).unwrap();
    }
}
