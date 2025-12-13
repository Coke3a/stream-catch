use anyhow::{Result, anyhow, bail};
use url::Url;

use crate::domain::value_objects::enums::platforms::Platform;

pub const MAX_LIVE_ACCOUNT_URL_LEN: usize = 2048;
pub const MAX_LIVE_ACCOUNT_ID_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedLiveAccountUrl {
    pub platform: Platform,
    pub account_id: String,
    pub canonical_url: String,
}

pub fn normalize_live_account_url(raw: &str) -> Result<NormalizedLiveAccountUrl> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("Invalid URL: empty input");
    }
    if trimmed.len() > MAX_LIVE_ACCOUNT_URL_LEN {
        bail!("Invalid URL: too long");
    }
    if trimmed.chars().any(|c| c.is_control()) {
        bail!("Invalid URL: contains control characters");
    }

    let lowered = trimmed.to_ascii_lowercase();
    let url_occurrences = lowered.matches("http://").count() + lowered.matches("https://").count();
    if url_occurrences > 1 {
        bail!("Invalid URL: multiple URLs detected");
    }

    let url = Url::parse(trimmed).map_err(|err| anyhow!("Invalid URL: {}", err))?;

    if url.scheme() != "https" {
        bail!("Invalid URL: only https scheme is allowed");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("Invalid URL: userinfo is not allowed");
    }
    if url.port().is_some() {
        bail!("Invalid URL: port is not allowed");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("Invalid URL: query strings and fragments are not allowed");
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("Invalid URL: missing host"))?;
    let platform = detect_platform(host).ok_or_else(|| anyhow!("Unsupported platform"))?;

    let (account_id, canonical_url) = normalize_url_for_platform(platform, &url)?;

    Ok(NormalizedLiveAccountUrl {
        platform,
        account_id,
        canonical_url,
    })
}

fn detect_platform(host: &str) -> Option<Platform> {
    match host.to_ascii_lowercase().as_str() {
        "www.tiktok.com" | "tiktok.com" => Some(Platform::TikTok),
        "www.twitch.tv" | "twitch.tv" => Some(Platform::Twitch),
        "www.bigo.tv" | "bigo.tv" => Some(Platform::Bigo),
        "kick.com" | "www.kick.com" => Some(Platform::Kick),
        "www.sooplive.com" | "sooplive.com" => Some(Platform::SoopLive),
        _ => None,
    }
}

fn normalize_url_for_platform(platform: Platform, url: &Url) -> Result<(String, String)> {
    match platform {
        Platform::TikTok => normalize_tiktok(url),
        Platform::Twitch => normalize_single_segment(url, "www.twitch.tv", UsernameRule::Strict),
        Platform::Bigo => normalize_single_segment(url, "www.bigo.tv", UsernameRule::Strict),
        Platform::Kick => normalize_single_segment(url, "kick.com", UsernameRule::Strict),
        Platform::SoopLive => {
            normalize_single_segment(url, "www.sooplive.com", UsernameRule::Strict)
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UsernameRule {
    Strict,
    TikTok,
}

fn normalize_single_segment(
    url: &Url,
    canonical_host: &'static str,
    rule: UsernameRule,
) -> Result<(String, String)> {
    let path = url.path().trim_matches('/');
    if path.is_empty() {
        bail!("Invalid URL: missing account id");
    }
    if path.contains('/') {
        bail!("Invalid URL: expected a single path segment");
    }

    validate_account_id(path, rule)?;

    let canonical_url = format!("https://{}/{}", canonical_host, path);
    Ok((path.to_string(), canonical_url))
}

fn normalize_tiktok(url: &Url) -> Result<(String, String)> {
    let segments: Vec<&str> = url.path().split('/').filter(|s| !s.is_empty()).collect();

    match segments.as_slice() {
        [user] | [user, "live"] => {
            if !user.starts_with('@') || user.len() < 2 {
                bail!("Invalid URL: invalid TikTok username segment");
            }
            let account_id = user.trim_start_matches('@');
            validate_account_id(account_id, UsernameRule::TikTok)?;

            let canonical_url = format!("https://www.tiktok.com/@{}/live", account_id);
            Ok((account_id.to_string(), canonical_url))
        }
        _ => bail!("Invalid URL: invalid TikTok URL format"),
    }
}

fn validate_account_id(account_id: &str, rule: UsernameRule) -> Result<()> {
    if account_id.is_empty() {
        bail!("Invalid URL: missing account id");
    }
    if account_id.len() > MAX_LIVE_ACCOUNT_ID_LEN {
        bail!("Invalid URL: account id too long");
    }
    if account_id.contains("..") {
        bail!("Invalid URL: invalid account id");
    }

    match rule {
        UsernameRule::Strict => {
            if !account_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                bail!("Invalid URL: invalid account id");
            }
        }
        UsernameRule::TikTok => {
            if account_id.starts_with('.') || account_id.ends_with('.') {
                bail!("Invalid URL: invalid account id");
            }
            if !account_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            {
                bail!("Invalid URL: invalid account id");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_kick_url_is_normalized() {
        let normalized = normalize_live_account_url("https://kick.com/nahyunworld").unwrap();
        assert_eq!(normalized.platform, Platform::Kick);
        assert_eq!(normalized.account_id, "nahyunworld");
        assert_eq!(normalized.canonical_url, "https://kick.com/nahyunworld");
    }

    #[test]
    fn valid_sooplive_url_is_normalized() {
        let normalized = normalize_live_account_url("https://www.sooplive.com/kiss2514").unwrap();
        assert_eq!(normalized.platform, Platform::SoopLive);
        assert_eq!(normalized.account_id, "kiss2514");
        assert_eq!(
            normalized.canonical_url,
            "https://www.sooplive.com/kiss2514"
        );
    }

    #[test]
    fn host_variants_are_canonicalized() {
        let normalized = normalize_live_account_url("https://www.kick.com/nahyunworld").unwrap();
        assert_eq!(normalized.canonical_url, "https://kick.com/nahyunworld");

        let normalized = normalize_live_account_url("https://sooplive.com/kiss2514").unwrap();
        assert_eq!(
            normalized.canonical_url,
            "https://www.sooplive.com/kiss2514"
        );
    }

    #[test]
    fn invalid_kick_urls_are_rejected() {
        for raw in [
            "https://kick.com/",
            "https://kick.com/a/b",
            "http://kick.com/nahyunworld",
            "https://kick.com/nahyunworld?x=1",
        ] {
            let err = normalize_live_account_url(raw).unwrap_err().to_string();
            assert!(
                err.contains("Invalid URL"),
                "expected invalid url error, got: {err}"
            );
        }
    }

    #[test]
    fn invalid_sooplive_urls_are_rejected() {
        for raw in ["https://www.sooplive.com/", "https://www.sooplive.com/a/b"] {
            let err = normalize_live_account_url(raw).unwrap_err().to_string();
            assert!(
                err.contains("Invalid URL"),
                "expected invalid url error, got: {err}"
            );
        }
    }

    #[test]
    fn concatenated_urls_are_rejected() {
        let err = normalize_live_account_url(
            "https://www.bigo.tv/butter4567https://www.bigo.tv/butter4567",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("Invalid URL"), "got: {err}");
    }

    #[test]
    fn input_containing_two_urls_is_rejected() {
        let err = normalize_live_account_url("https://kick.com/nahyunworld https://kick.com/other")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Invalid URL"), "got: {err}");
    }

    #[test]
    fn host_must_match_exactly() {
        let err = normalize_live_account_url("https://kick.com.evil.com/nahyunworld")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unsupported platform"), "got: {err}");
    }
}
