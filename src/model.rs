use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Status {
    Connected(String),
    Connecting,
    Disconnecting,
    Disconnected,
    #[default]
    Unknown,
}

impl Status {
    pub fn label(&self) -> String {
        match self {
            Self::Connected(city) => format!("연결됨 · {}", crate::places::display(city)),
            Self::Connecting => "연결 중…".into(),
            Self::Disconnecting => "연결 해제 중…".into(),
            Self::Disconnected => "연결되지 않음".into(),
            Self::Unknown => "상태 확인 불가".into(),
        }
    }
    pub fn connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }
}

#[derive(Clone, Debug)]
pub struct Location {
    pub iso: String,
    pub country: String,
    pub city: String,
    pub ping: Option<u32>,
}

impl Location {
    pub fn key(&self) -> String {
        format!("{}:{}", self.iso, self.city)
    }
}

static ANSI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\))").unwrap()
});
static CONNECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)connected to\s+(.+?)\s+in\s+(?:TUN|SOCKS)\s+mode").unwrap());
static COLUMNS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s{2,}|\t+").unwrap());

pub fn clean(text: &str) -> String {
    ANSI.replace_all(text, "").trim().to_owned()
}

pub fn parse_status(text: &str) -> Status {
    let text = clean(text);
    let low = text.to_lowercase();
    if low.contains("not connected")
        || low.contains("vpn is disconnected")
        || low.contains("no active connection")
    {
        return Status::Disconnected;
    }
    if low.contains("disconnecting") {
        return Status::Disconnecting;
    }
    if let Some(captures) = CONNECTED.captures(&text) {
        return Status::Connected(captures[1].trim().into());
    }
    if low.contains("connecting") {
        return Status::Connecting;
    }
    Status::Unknown
}

pub fn parse_locations(text: &str) -> Result<Vec<Location>, String> {
    let mut result = Vec::new();
    for line in clean(text).lines() {
        let columns: Vec<_> = COLUMNS.split(line.trim()).collect();
        if columns.len() < 3
            || columns[0].len() != 2
            || !columns[0].bytes().all(|c| c.is_ascii_alphabetic())
        {
            continue;
        }
        result.push(Location {
            iso: columns[0].to_uppercase(),
            country: columns[1].into(),
            city: columns[2].into(),
            ping: columns.get(3).and_then(|v| {
                v.trim_start_matches(|c: char| !c.is_ascii_digit())
                    .split(|c: char| !c.is_ascii_digit())
                    .next()?
                    .parse()
                    .ok()
            }),
        });
    }
    if result.is_empty() {
        Err("서버 목록을 읽지 못했습니다. CLI 로그인과 응답 형식을 확인하세요.".into())
    } else {
        Ok(result)
    }
}

pub fn config_value(text: &str, key: &str) -> Option<String> {
    clean(text).lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(key)
            .then(|| value.trim().to_owned())
    })
}

#[derive(Debug, PartialEq)]
pub struct Subscription {
    pub logged_in: bool,
    pub identity: Option<String>,
    pub tier: Option<String>,
    pub devices: Option<String>,
    pub expires: Option<String>,
}

pub fn parse_subscription(text: &str) -> Result<Subscription, String> {
    let text = clean(text);
    let low = text.to_lowercase();
    if low.contains("not logged in")
        || low.contains("please log in")
        || low.contains("not authenticated")
    {
        return Ok(Subscription {
            logged_in: false,
            identity: None,
            tier: None,
            devices: None,
            expires: None,
        });
    }
    let capture = |pattern: &str| -> Option<String> {
        Regex::new(pattern)
            .ok()?
            .captures(&text)
            .map(|c| c[1].trim().to_owned())
    };
    let identity = capture(r"(?im)logged\s+in\s+as\s+([^\r\n]+)");
    if identity.is_none() {
        return Err("구독 응답 형식을 인식하지 못했습니다. 아래 응답 원문을 확인하세요.".into());
    }
    Ok(Subscription {
        logged_in: true,
        identity,
        tier: capture(r"(?i)using\s+the\s+([a-z]+)\s+version"),
        devices: capture(r"(?i)up\s+to\s+(\d+)\s+devices"),
        expires: capture(r"(?i)valid\s+until\s+(\d{4}-\d{2}-\d{2})"),
    })
}

impl Subscription {
    pub fn display(&self) -> String {
        if !self.logged_in {
            return "로그인되지 않았습니다. 공식 CLI 로그인 버튼을 눌러주세요.".into();
        }
        let tier = match self.tier.as_deref().map(str::to_ascii_uppercase).as_deref() {
            Some("PREMIUM") => "프리미엄".to_owned(),
            Some("FREE") => "무료".to_owned(),
            Some(value) => value.to_owned(),
            None => "확인 필요".to_owned(),
        };
        format!(
            "계정: {}\n구독: {}\n기기 수: {}\n만료일: {}",
            self.identity.as_deref().unwrap_or("확인 필요"),
            tier,
            self.devices
                .as_ref()
                .map(|v| format!("최대 {v}대"))
                .unwrap_or_else(|| "확인 필요".into()),
            self.expires.as_deref().unwrap_or("확인 필요")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiword_city_and_ansi_status() {
        assert_eq!(
            parse_status("\x1b[32mConnected to New York in TUN mode, running on tun0\x1b[0m"),
            Status::Connected("New York".into())
        );
    }

    #[test]
    fn unknown_output_does_not_imply_disconnected() {
        assert_eq!(parse_status("unexpected success message"), Status::Unknown);
        assert_eq!(parse_status("VPN is not connected"), Status::Disconnected);
        assert_eq!(parse_status("Disconnecting…"), Status::Disconnecting);
    }

    #[test]
    fn location_columns_preserve_multiword_names_and_unknown_latency() {
        let locations = parse_locations("ISO  COUNTRY  CITY  PING\nUS  United States  New York  24 ms\nKR  South Korea  Seoul  N/A").unwrap();
        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0].city, "New York");
        assert_eq!(locations[0].ping, Some(24));
        assert_eq!(locations[1].ping, None);
    }

    #[test]
    fn invalid_location_output_is_an_error() {
        assert!(parse_locations("Authentication required").is_err());
    }

    #[test]
    fn dns_value_preserves_colons() {
        assert_eq!(
            config_value(
                "DNS upstream: https://dns.example/dns-query",
                "DNS upstream"
            ),
            Some("https://dns.example/dns-query".into())
        );
        assert_eq!(config_value("Unrecognized output", "Protocol"), None);
    }
}
