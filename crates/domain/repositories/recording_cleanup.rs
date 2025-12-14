use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::recordings::RecordingEntity;

#[async_trait]
pub trait RecordingCleanupRepository {
    async fn list_expired_ready_recordings(
        &self,
        older_than: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<RecordingEntity>>;

    async fn mark_recording_expired_deleted(&self, recording_id: Uuid) -> Result<Uuid>;
}
