use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::{entities::recordings::{InsertRecordingEntity, RecordingEntity}, value_objects::enums::recording_statuses::RecordingStatus};

#[async_trait]
#[automock]
pub trait RecordingJobRepository {
    async fn find_recording_by_live_account_and_status(
        &self,
        platform: String,
        account_id: Uuid,
        status: RecordingStatus,
    ) -> Result<Option<RecordingEntity>>;
    async fn insert(
        &self,
        insert_recording_entity: InsertRecordingEntity,
    ) -> Result<Uuid>;
    async fn update_live_end(
        &self,
        recording_id: Uuid,
        duration: i64,
    ) -> Result<Uuid>;
    async fn update_live_transmux_finish(
        &self,
        recording_id: Uuid,
        storage_path: String
    ) -> Result<Uuid>;
    async fn update_file_uploading(
        &self,
        recording_id: Uuid,
    ) -> Result<Uuid>;
}


