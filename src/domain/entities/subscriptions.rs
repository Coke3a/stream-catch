use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infrastructure::postgres::schema::subscriptions;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = subscriptions)]
pub struct SubscriptionEntity {
    pub id: i64,
    pub user_id: Uuid,
    pub plan_id: i64,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub billing_mode: String,
    pub default_payment_method_id: Option<i64>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = subscriptions)]
pub struct InsertSubscriptionEntity {
    pub user_id: Uuid,
    pub plan_id: i64,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub billing_mode: String,
    pub default_payment_method_id: Option<i64>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub status: String,
}
