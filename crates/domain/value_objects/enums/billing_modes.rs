use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BillingMode {
    Recurring,
    Manual,
}

impl BillingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingMode::Recurring => "recurring",
            BillingMode::Manual => "manual",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "recurring" => Some(BillingMode::Recurring),
            "manual" => Some(BillingMode::Manual),
            _ => None,
        }
    }
}

impl Display for BillingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
