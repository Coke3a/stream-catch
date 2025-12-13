use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Platform {
    TikTok,
    Twitch,
    Bigo,
    Kick,
    SoopLive,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let platform = match self {
            Platform::TikTok => "tiktok",
            Platform::Twitch => "twitch",
            Platform::Bigo => "bigo",
            Platform::Kick => "kick",
            Platform::SoopLive => "sooplive",
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
            "kick" => Ok(Platform::Kick),
            "sooplive" => Ok(Platform::SoopLive),
            other => Err(format!("Unsupported platform: {}", other)),
        }
    }
}
