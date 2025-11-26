use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::entities::{
    jobs::InsertJobEntity,
    recordings::{InsertRecordingEntity, UpdateRecordingEntity},
};

#[async_trait]
#[automock]
pub trait RecordingJobRepository {
    async fn webhook_recording_start(
        &self,
        insert_recording_entity: InsertRecordingEntity,
    ) -> Result<Uuid>;
    async fn webhook_recording_end(
        &self,
        update_recording_entity: UpdateRecordingEntity,
    ) -> Result<Uuid>;
    async fn upload_recording_job_start(&self, insert_job_entity: InsertJobEntity) -> Result<Uuid>;
    async fn upload_recording_job_end(
        &self,
        update_recording_entity: UpdateRecordingEntity,
    ) -> Result<Uuid>;
}
