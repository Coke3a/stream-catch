use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{    
    value_objects::enums::subscription_statuses::SubscriptionStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionModel {
    pub id: i64,
    pub user_id: Uuid,
    pub plan_id: i64,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub billing_mode: String,
    pub default_payment_method_id: Option<i64>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub status: SubscriptionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertSubscriptionModel {
    pub plan_id: i64,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanModel {
    pub id: i64,
    pub name: Option<String>,
    pub price_minor: i32,
    pub duration_days: i32,
    pub features: Value,
    pub is_active: bool,
}
