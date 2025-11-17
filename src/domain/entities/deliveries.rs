use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infrastructure::postgres::schema::deliveries;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = deliveries)]
pub struct DeliveryEntity {
    pub id: i64,
    pub recording_id: i64,
    pub user_id: Uuid,
    pub via: String,
    pub delivered_at: Option<DateTime<Utc>>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = deliveries)]
pub struct InsertDeliveryEntity {
    pub recording_id: i64,
    pub user_id: Uuid,
    pub via: String,
    pub delivered_at: Option<DateTime<Utc>>,
    pub status: String,
    pub error: Option<String>,
}
