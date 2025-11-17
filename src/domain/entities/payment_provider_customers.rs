use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use crate::infrastructure::postgres::schema::payment_provider_customers;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = payment_provider_customers)]
pub struct PaymentProviderCustomerEntity {
    pub id: i64,
    pub user_id: Uuid,
    pub provider: String,
    pub customer_ref: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = payment_provider_customers)]
pub struct InsertPaymentProviderCustomerEntity {
    pub user_id: Uuid,
    pub provider: String,
    pub customer_ref: String,
    pub metadata: Value,
}
