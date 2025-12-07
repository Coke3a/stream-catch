use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentStatus {
    RequiresAction,
    Processing,
    Succeeded,
    Failed,
    Canceled,
}

impl PaymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentStatus::RequiresAction => "requires_action",
            PaymentStatus::Processing => "processing",
            PaymentStatus::Succeeded => "succeeded",
            PaymentStatus::Failed => "failed",
            PaymentStatus::Canceled => "canceled",
        }
    }
}

impl Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
