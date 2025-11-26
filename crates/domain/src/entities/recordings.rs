use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::recordings;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = recordings)]
pub struct RecordingEntity {
    pub id: Uuid,
    pub live_account_id: Uuid,
    pub recording_key: Option<String>,
    pub title: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_path: Option<String>,
    pub storage_temp_path: Option<String>,
    pub status: String,
    pub poster_storage_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = recordings)]
pub struct InsertRecordingEntity {
    pub live_account_id: Uuid,
    pub title: Option<String>,
    pub poster_storage_path: Option<String>,
    pub started_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
