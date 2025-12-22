use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infra::db::postgres::schema::invoices;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = invoices)]
pub struct InvoiceEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub plan_id: Uuid,
    pub amount_minor: i32,
    pub currency: String,
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
    pub subscription_id: Option<Uuid>,
    pub plan_id: Uuid,
    pub amount_minor: i32,
    pub currency: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
}
