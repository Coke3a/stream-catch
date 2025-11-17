use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    entities::recordings::{InsertRecordingEntity, UpdateRecordingEntity},
    value_objects::enums::recording_statuses::RecordingStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingModel {
    pub id: i64,
    pub live_account_id: i64,
    pub recording_key: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: RecordingStatus,
    pub poster_storage_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsertRecordingModel {
    pub live_account_id: i64,
    pub recording_key: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: RecordingStatus,
    pub poster_storage_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InsertRecordingModel {
    pub fn to_entity(&self) -> InsertRecordingEntity {
        InsertRecordingEntity {
            live_account_id: self.live_account_id,
            recording_key: self.recording_key.clone(),
            started_at: self.started_at,
            ended_at: self.ended_at,
            duration_sec: self.duration_sec,
            size_bytes: self.size_bytes,
            storage_prefix: self.storage_prefix.clone(),
            status: RecordingStatus::Processing.to_string(),
            poster_storage_path: self.poster_storage_path.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateRecordingModel {
    pub recording_key: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_prefix: Option<String>,
    pub status: RecordingStatus,
    pub poster_storage_path: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl UpdateRecordingModel {
    pub fn to_entity(&self) -> UpdateRecordingEntity {
        UpdateRecordingEntity {
            recording_key: self.recording_key.clone(),
            started_at: self.started_at,
            ended_at: self.ended_at,
            duration_sec: self.duration_sec,
            size_bytes: self.size_bytes,
            storage_prefix: self.storage_prefix.clone(),
            status: self.status.to_string(),
            poster_storage_path: self.poster_storage_path.clone(),
            updated_at: Utc::now(),
        }
    }
}
