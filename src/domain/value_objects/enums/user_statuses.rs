use std::fmt::Display;
use serde::{Deserialize, Serialize};


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