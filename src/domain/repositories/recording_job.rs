use anyhow::Result;
use axum::async_trait;
use mockall::automock;

use crate::domain::entities::{jobs::InsertJobEntity, recordings::{InsertRecordingEntity, UpdateRecordingEntity}};

#[async_trait]
#[automock]
pub trait RecordingJobRepository {
    async fn webhook_recording_start(&self, insert_recording_entity: InsertRecordingEntity) -> Result<i64>;
    async fn webhook_recording_end(&self, update_recording_entity: UpdateRecordingEntity) -> Result<i64>;
    async fn upload_recording_job_start(&self, insert_job_entity: InsertJobEntity) -> Result<i64>;
    async fn upload_recording_job_end(&self, update_recording_entity: UpdateRecordingEntity) -> Result<i64>;
}
