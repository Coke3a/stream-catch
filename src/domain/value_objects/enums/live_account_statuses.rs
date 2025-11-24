use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LiveAccountStatus {
    #[default]
    Active,
    Paused,
    Error,
}

impl Display for LiveAccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let follow_status = match self {
            LiveAccountStatus::Active => "active",
            LiveAccountStatus::Paused => "paused",
            LiveAccountStatus::Error => "error",
        };
        write!(f, "{}", follow_status)
    }
}
