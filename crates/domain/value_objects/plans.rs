use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Fixed UUID representing the free plan.
pub const FREE_PLAN_ID: Uuid = Uuid::nil();

/// Limits and feature flags attached to a plan. Stored as JSONB in the database.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PlanFeatures {
    #[serde(default)]
    pub max_follows: Option<i64>,

    #[serde(default)]
    pub retention_days: Option<i32>,

    #[serde(default)]
    pub max_concurrent_recordings: Option<i32>,

    #[serde(default)]
    pub priority_support: Option<bool>,

    #[serde(default)]
    pub custom_branding: Option<bool>,
}

impl PlanFeatures {
    pub fn max_follows_or_default(&self) -> i64 {
        self.max_follows.unwrap_or(0)
    }

    pub fn retention_days_or_default(&self) -> i32 {
        self.retention_days.unwrap_or(0)
    }

    pub fn max_concurrent_recordings_or_default(&self) -> i32 {
        self.max_concurrent_recordings.unwrap_or(1)
    }

    pub fn has_priority_support(&self) -> bool {
        self.priority_support.unwrap_or(false)
    }

    pub fn has_custom_branding(&self) -> bool {
        self.custom_branding.unwrap_or(false)
    }
}
