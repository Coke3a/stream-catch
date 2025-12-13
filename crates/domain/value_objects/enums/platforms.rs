use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Platform {
    TikTok,
    Twitch,
    Bigo,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let platform = match self {
            Platform::TikTok => "tiktok",
            Platform::Twitch => "twitch",
            Platform::Bigo => "bigo",
        };
        write!(f, "{}", platform)
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "tiktok" => Ok(Platform::TikTok),
            "twitch" => Ok(Platform::Twitch),
            "bigo" => Ok(Platform::Bigo),
            other => Err(format!("Unsupported platform: {}", other)),
        }
    }
}
