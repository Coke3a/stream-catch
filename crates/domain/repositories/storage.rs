use crate::domain::entities::recordings::RecordingEntity;
use crate::domain::value_objects::storage::UploadResult;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait StorageClient {
    async fn upload_recording(
        &self,
        local_path: &str,
        recording: &RecordingEntity,
    ) -> Result<UploadResult>;
}

#[async_trait]
pub trait CoverStorageClient {
    async fn upload_cover(
        &self,
        recording_id: Uuid,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<String>;
}
