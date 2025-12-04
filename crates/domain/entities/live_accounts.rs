use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infra::db::postgres::schema::live_accounts;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = live_accounts)]
pub struct LiveAccountEntity {
    pub id: Uuid,
    pub platform: String, // string platform name enum in crates/domain/src/value_objects/enums/platforms.rs example: bigo, twitch
    pub account_id: String,
    pub canonical_url: String, // example: https://www.bigo.tv/sai239233 or https://www.twitch.tv/arii
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
