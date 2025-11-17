use std::fmt::Display;
use serde::{Deserialize, Serialize};

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
            Platform::YouTube => "YouTube",
            Platform::TikTok => "TikTok",
            Platform::Twitch => "Twitch",
            Platform::Bigo => "Bigo",
        };
        write!(f, "{}", platform)
    }
}
