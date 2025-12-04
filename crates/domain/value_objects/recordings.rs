use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::entities::recordings::InsertRecordingEntity,
    domain::value_objects::enums::{
        platforms::Platform, recording_statuses::RecordingStatus, sort_order::SortOrder,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingModel {
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
    pub status: RecordingStatus,
    pub poster_storage_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsertRecordingModel {
    pub live_account_id: Uuid,
    pub poster_storage_path: Option<String>,
    pub title: Option<String>,
}

impl InsertRecordingModel {
    pub fn to_entity(&self) -> InsertRecordingEntity {
        let now = Utc::now();
        InsertRecordingEntity {
            live_account_id: self.live_account_id,
            title: self.title.clone(),
            poster_storage_path: self.poster_storage_path.clone(),
            started_at: now,
            status: RecordingStatus::LiveRecording.to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ListRecordingsFilter {
    pub live_account_id: Option<Uuid>,
    pub platform: Option<Platform>,
    pub status: Option<RecordingStatus>,
    pub limit: Option<i64>,
    pub sort_order: SortOrder,
}
