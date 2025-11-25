use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FollowStatus {
    #[default]
    Active,
    Inactive,
    TemporaryInactive,
}

impl Display for FollowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let follow_status = match self {
            FollowStatus::Active => "active",
            FollowStatus::Inactive => "inactive",
            FollowStatus::TemporaryInactive => "temporary_inactive",
        };
        write!(f, "{}", follow_status)
    }
}
