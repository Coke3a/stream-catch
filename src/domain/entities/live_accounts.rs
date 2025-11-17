use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::infrastructure::postgres::schema::live_accounts;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = live_accounts)]
pub struct LiveAccountEntity {
    pub id: i64,
    pub platform: String,
    pub account_id: String,
    pub canonical_url: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = live_accounts)]
pub struct InsertLiveAccountEntity {
    pub platform: String,
    pub account_id: String,
    pub canonical_url: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

