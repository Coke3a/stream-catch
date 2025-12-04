use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infra::db::postgres::schema::payment_methods;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethodEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub method_type: String,
    pub pm_ref: String,
    pub brand: Option<String>,
    pub last4: Option<String>,
    pub exp_month: Option<i32>,
    pub exp_year: Option<i32>,
    pub status: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = payment_methods)]
pub struct InsertPaymentMethodEntity {
    pub user_id: Uuid,
    pub provider: String,
    pub method_type: String,
    pub pm_ref: String,
    pub brand: Option<String>,
    pub last4: Option<String>,
    pub exp_month: Option<i32>,
    pub exp_year: Option<i32>,
    pub status: String,
    pub is_default: bool,
}
