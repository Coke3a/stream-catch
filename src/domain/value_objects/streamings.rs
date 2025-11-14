use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    entities::recordings::{InsertRecordingEntity, UpdateRecordingEntity},
    value_objects::recording_statuses::RecordingStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingModel {
    pub id: i64,
    pub live_account_id: i64,
    pub started_at: Option<NaiveDateTime>,
    pub ended_at: Option<NaiveDateTime>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: RecordingStatus,
    pub poster_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsertRecordingModel {
    pub live_account_id: i64,
    pub started_at: Option<NaiveDateTime>,
    pub ended_at: Option<NaiveDateTime>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: RecordingStatus,
    pub poster_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl InsertRecordingModel {
    pub fn to_entity(&self) -> InsertRecordingEntity {
        InsertRecordingEntity {
            live_account_id: self.live_account_id,
            started_at: self.started_at,
            ended_at: self.ended_at,
            duration_sec: self.duration_sec,
            size_bytes: self.size_bytes,
            storage_prefix: self.storage_prefix.clone(),
            status: RecordingStatus::Recording.to_string(),
            poster_key: self.poster_key.clone(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateRecordingModel {
    pub started_at: Option<NaiveDateTime>,
    pub ended_at: Option<NaiveDateTime>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: RecordingStatus,
    pub poster_key: Option<String>,
    pub updated_at: NaiveDateTime,
}

impl UpdateRecordingModel {
    pub fn to_entity(&self) -> UpdateRecordingEntity {
        UpdateRecordingEntity {
            started_at: self.started_at,
            ended_at: self.ended_at,
            duration_sec: self.duration_sec,
            size_bytes: self.size_bytes,
            storage_prefix: self.storage_prefix.clone(),
            status: self.status.to_string(),
            poster_key: self.poster_key.clone(),
            updated_at: Utc::now().naive_utc(),
        }
    }
}
