use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infrastructure::postgres::schema::payments;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = payments)]
pub struct PaymentEntity {
    pub id: i64,
    pub invoice_id: i64,
    pub user_id: Uuid,
    pub provider: String,
    pub method_type: String,
    pub payment_method_id: Option<i64>,
    pub amount_minor: i32,
    pub status: String,
    pub provider_payment_id: Option<String>,
    pub provider_session_ref: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = payments)]
pub struct InsertPaymentEntity {
    pub invoice_id: i64,
    pub user_id: Uuid,
    pub provider: String,
    pub method_type: String,
    pub payment_method_id: Option<i64>,
    pub amount_minor: i32,
    pub status: String,
    pub provider_payment_id: Option<String>,
    pub provider_session_ref: Option<String>,
    pub error: Option<String>,
}
