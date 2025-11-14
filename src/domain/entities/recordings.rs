use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::infrastructure::postgres::schema::recordings;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = recordings)]
pub struct RecordingEntity {
    pub id: i64,
    pub live_account_id: i64,
    pub started_at: Option<NaiveDateTime>,
    pub ended_at: Option<NaiveDateTime>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: String,
    pub poster_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = recordings)]
pub struct InsertRecordingEntity {
    pub live_account_id: i64,
    pub started_at: Option<NaiveDateTime>,
    pub ended_at: Option<NaiveDateTime>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: String,
    pub poster_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, AsChangeset, Queryable)]
#[diesel(table_name = recordings)]
pub struct UpdateRecordingEntity {
    pub started_at: Option<NaiveDateTime>,
    pub ended_at: Option<NaiveDateTime>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: String,
    pub poster_key: Option<String>,
    pub updated_at: NaiveDateTime,
}