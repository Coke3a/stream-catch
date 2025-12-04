use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use domain::entities::recordings::RecordingEntity;

#[derive(Debug, Clone)]
pub struct UploadResult {
    pub remote_prefix: String,
    pub size_bytes: i64,
    pub duration_sec: i32,
}

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
