use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SubscriptionStatus {
    #[default]
    Active,
    Pending,
    PastDue,
    Canceled,
    Expired,
}

impl Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            SubscriptionStatus::Active => "active",
            SubscriptionStatus::Pending => "pending",
            SubscriptionStatus::PastDue => "past_due",
            SubscriptionStatus::Canceled => "canceled",
            SubscriptionStatus::Expired => "expired",
        };
        write!(f, "{}", status)
    }
}

impl SubscriptionStatus {
    pub fn from_str(value: &str) -> Self {
        match value {
            "active" => SubscriptionStatus::Active,
            "pending" => SubscriptionStatus::Pending,
            "past_due" => SubscriptionStatus::PastDue,
            "canceled" => SubscriptionStatus::Canceled,
            "expired" => SubscriptionStatus::Expired,
            _ => SubscriptionStatus::Expired,
        }
    }
}
