use std::fmt::Display;
use serde::{Deserialize, Serialize};


#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserStatus {
    #[default]
    Active,
    Inactive,
    Deleted,
}

impl Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            UserStatus::Active => "Active",
            UserStatus::Inactive => "Inactive",
            UserStatus::Deleted => "Deleted",
        };
        write!(f, "{}", status)
    }
}