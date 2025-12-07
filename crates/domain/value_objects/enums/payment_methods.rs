use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentMethod {
    Card,
    PromptPay,
}

impl PaymentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentMethod::Card => "card",
            PaymentMethod::PromptPay => "promptpay",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "card" => Some(PaymentMethod::Card),
            "promptpay" => Some(PaymentMethod::PromptPay),
            _ => None,
        }
    }
}

impl Display for PaymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
