use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Platform {
    YouTube,
    TikTok,
    Twitch,
    Bigo,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let platform = match self {
            Platform::YouTube => "youtube",
            Platform::TikTok => "tiktok",
            Platform::Twitch => "twitch",
            Platform::Bigo => "bigo",
        };
        write!(f, "{}", platform)
    }
}
