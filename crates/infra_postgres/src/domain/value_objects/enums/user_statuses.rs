use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserStatus {
    #[default]
    Active,
    Blocked,
    Inactive,
}

impl Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            UserStatus::Active => "active",
            UserStatus::Blocked => "blocked",
            UserStatus::Inactive => "inactive",
        };
        write!(f, "{}", status)
    }
}
