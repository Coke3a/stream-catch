use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::{
    domain::entities::{
        recordings::{
            RecordingEntity,
        },
    },
};

#[async_trait]
#[automock]
pub trait RecordingUploadRepository {
    async fn find_recording_by_id(&self, recording_id: Uuid) -> Result<Option<RecordingEntity>>;

    async fn mark_recording_ready(
        &self,
        recording_id: Uuid,
        storage_path: String,
        size_bytes: i64,
        duration_sec: i32,
    ) -> Result<Uuid>;
}