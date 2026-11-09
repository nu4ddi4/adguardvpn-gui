use crate::model;
use std::net::IpAddr;

pub fn validate(input: &str) -> Result<String, String> {
    let text = input.trim().trim_end_matches('.').to_ascii_lowercase();
    let invalid = || {
        "도메인·와일드카드(*.example.com)·IP·CIDR을 입력하세요. URL 경로와 포트는 제외하세요."
            .to_owned()
    };
    if let Some((address, bits)) = text.split_once('/') {
        let ip: IpAddr = address.parse().map_err(|_| invalid())?;
        let bits: u8 = bits.parse().map_err(|_| invalid())?;
        if bits > if ip.is_ipv4() { 32 } else { 128 } {
            return Err(invalid());
        }
        return Ok(text);
    }
    if text.parse::<IpAddr>().is_ok() {
        return Ok(text);
    }
    let domain = text.strip_prefix("*.").unwrap_or(&text);
    if domain.is_empty()
        || domain.len() > 253
        || !domain.is_ascii()
        || !domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
    {
        return Err(invalid());
    }
    Ok(text)
}

pub fn parse_mode(text: &str) -> Result<String, String> {
    let clean = model::clean(text).to_ascii_lowercase();
    for mode in ["general", "selective"] {
        if clean == mode || clean.contains(&format!("exclusion mode is {mode}")) {
            return Ok(mode.into());
        }
    }
    Err("현재 사이트 적용 모드를 읽지 못했습니다. 새로 고침 후 다시 시도하세요.".into())
}

pub fn parse_entries(text: &str) -> Result<Vec<String>, String> {
    let clean = model::clean(text);
    let mut known = false;
    let mut entries = Vec::new();
    for line in clean.lines().map(str::trim).filter(|s| !s.is_empty()) {
        let low = line.to_ascii_lowercase();
        if low.starts_with("exclusions for ")
            || low.starts_with("no exclusions")
            || low.contains("list is empty")
        {
            known = true;
            continue;
        }
        let entry = validate(line).map_err(|_| {
            "사이트 목록 응답 형식을 인식하지 못했습니다. 기존 목록을 유지합니다.".to_owned()
        })?;
        entries.push(entry);
    }
    if entries.is_empty() && !known && !clean.is_empty() {
        return Err("사이트 목록을 읽지 못했습니다.".into());
    }
    entries.sort();
    entries.dedup();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_boundaries() {
        for value in [
            "example.com",
            "*.example.com",
            "10.0.0.0/8",
            "2001:db8::/32",
        ] {
            assert!(validate(value).is_ok());
        }
        for value in [
            "--help",
            "https://example.com/path",
            "a.com:443",
            "10.0.0.0/33",
            "",
            "a..com",
        ] {
            assert!(validate(value).is_err(), "{value}");
        }
    }
    #[test]
    fn cli_modes_and_lists() {
        assert_eq!(
            parse_mode("Current exclusion mode is selective").unwrap(),
            "selective"
        );
        assert!(parse_mode("Authentication failed").is_err());
        assert_eq!(
            parse_entries("Exclusions for general mode:\nexample.com\n*.example.net")
                .unwrap()
                .len(),
            2
        );
        assert!(parse_entries("Authentication failed").is_err());
    }
}
