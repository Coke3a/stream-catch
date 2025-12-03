use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::entities::jobs::JobEntity;

#[async_trait]
#[automock]
pub trait JobRepository {
    async fn enqueue_recording_upload_job(
        &self,
        recording_id: Uuid,
        local_path: String,
    ) -> Result<Uuid>;

    async fn lock_next_recording_upload_job(&self) -> Result<Option<JobEntity>>;

    async fn mark_job_done(&self, job_id: Uuid) -> Result<()>;

    async fn mark_job_failed(&self, job_id: Uuid, err: &str, max_attempts: i32) -> Result<()>;
}
