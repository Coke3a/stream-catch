use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LiveAccountStatus {
    #[default]
    Synced,
    Unsynced,
    Error,
}

impl Display for LiveAccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let follow_status = match self {
            LiveAccountStatus::Synced => "synced",
            LiveAccountStatus::Unsynced => "unsynced",
            LiveAccountStatus::Error => "error",
        };
        write!(f, "{}", follow_status)
    }
}
