use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infrastructure::postgres::schema::invoices;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = invoices)]
pub struct InvoiceEntity {
    pub id: i64,
    pub user_id: Uuid,
    pub subscription_id: Option<i64>,
    pub plan_id: i64,
    pub amount_minor: i32,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = invoices)]
pub struct InsertInvoiceEntity {
    pub user_id: Uuid,
    pub subscription_id: Option<i64>,
    pub plan_id: i64,
    pub amount_minor: i32,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
}
