use anyhow::Result;
use axum::async_trait;
use mockall::automock;

use crate::domain::entities::recordings::{InsertRecordingEntity, UpdateRecordingEntity};

#[async_trait]
#[automock]
pub trait RecordingJobRepository {
    async fn webhook_recording_start(&self, insert_recording_entity: InsertRecordingEntity) -> Result<i64>;
    async fn webhook_recording_end(&self, update_recording_entity: UpdateRecordingEntity) -> Result<i64>;
    async fn upload_recording_start(&self, ) -> Result<i64>;
    async fn upload_recording_end(&self, ) -> Result<i64>;
}
