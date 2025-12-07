use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::plans::PlanEntity;
use crate::domain::value_objects::enums::{billing_modes::BillingMode, subscription_statuses::SubscriptionStatus};
use crate::domain::value_objects::plans::PlanFeatures;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionModel {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub billing_mode: String,
    pub default_payment_method_id: Option<Uuid>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub provider_subscription_id: Option<String>,
    pub status: SubscriptionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertSubscriptionModel {
    pub plan_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanModel {
    pub id: Uuid,
    pub name: Option<String>,
    pub price_minor: i32,
    pub duration_days: i32,
    pub features: PlanFeatures,
    pub is_active: bool,
    pub stripe_price_recurring: Option<String>,
    pub stripe_price_one_time_card: Option<String>,
    pub stripe_price_one_time_promptpay: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlanDto {
    pub id: Uuid,
    pub name: Option<String>,
    pub price_minor: i32,
    pub duration_days: i32,
    pub features: PlanFeatures,
}

impl From<PlanEntity> for PlanDto {
    fn from(value: PlanEntity) -> Self {
        Self {
            id: value.id,
            name: value.name,
            price_minor: value.price_minor,
            duration_days: value.duration_days,
            features: value.features,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CurrentSubscriptionDto {
    pub plan_id: Uuid,
    pub plan_name: Option<String>,
    pub billing_mode: BillingMode,
    pub status: SubscriptionStatus,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub features: PlanFeatures,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan_id: Uuid,
    pub billing_mode: String,
    pub payment_method: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCheckoutResponse {
    pub checkout_url: String,
}
