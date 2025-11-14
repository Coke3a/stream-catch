use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FollowStatus {
    #[default]
    Active,
    Inactive,
    TemporaryInactive,
}

impl Display for FollowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let follow_status = match self {
            FollowStatus::Active => "Active",
            FollowStatus::Inactive => "Inactive",
            FollowStatus::TemporaryInactive => "TemporaryInactive",
        };
        write!(f, "{}", follow_status)
    }
}
