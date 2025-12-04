use std::{env, path::Path};

use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use mime_guess::MimeGuess;
use tokio::fs;

use crate::domain::{entities::recordings::RecordingEntity, repositories::storage::StorageClient, value_objects::storage::UploadResult};

use super::s3::{S3Config, build_s3_client};

#[derive(Clone, Debug)]
pub struct B2StorageConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub key_id: String,
    pub application_key: String,
    pub key_prefix: String,
}

impl B2StorageConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            endpoint: env::var("B2_ENDPOINT").context("B2_ENDPOINT is required")?,
            region: env::var("B2_REGION").context("B2_REGION is required")?,
            bucket: env::var("B2_BUCKET").context("B2_BUCKET is required")?,
            key_id: env::var("B2_KEY_ID").context("B2_KEY_ID is required")?,
            application_key: env::var("B2_APPLICATION_KEY")
                .context("B2_APPLICATION_KEY is required")?,
            key_prefix: env::var("B2_KEY_PREFIX").unwrap_or_else(|_| "recordings".to_string()),
        })
    }
}

pub struct B2StorageClient {
    client: aws_sdk_s3::Client,
    bucket: String,
    key_prefix: String,
}

impl B2StorageClient {
    pub async fn new(config: B2StorageConfig) -> Result<Self> {
        let s3_client = build_s3_client(&S3Config {
            endpoint: config.endpoint,
            region: config.region,
            access_key: config.key_id,
            secret_key: config.application_key,
            force_path_style: true,
            connect_timeout_secs: 10,
        })
        .await
        .context("failed to build B2 s3 client")?;

        let prefix = normalize_prefix(&config.key_prefix);

        Ok(Self {
            client: s3_client,
            bucket: config.bucket,
            key_prefix: prefix,
        })
    }
}

#[async_trait]
impl StorageClient for B2StorageClient {
    async fn upload_recording(
        &self,
        local_path: &str,
        recording: &RecordingEntity,
    ) -> Result<UploadResult> {
        let path = Path::new(local_path);
        if !path.exists() {
            anyhow::bail!("local file does not exist: {}", local_path);
        }

        let metadata = fs::metadata(path)
            .await
            .with_context(|| format!("failed to read metadata for {}", local_path))?;
        let size_bytes = i64::try_from(metadata.len()).context("recording size is too large")?;

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .filter(|ext| !ext.is_empty())
            .unwrap_or("mp4");

        let object_name = format!("recording-{}.{}", recording.id, extension);
        let object_key = format!("{}{}", self.key_prefix, object_name);

        let content_type = MimeGuess::from_path(path)
            .first_raw()
            .unwrap_or("video/mp4")
            .to_string();

        let body = ByteStream::from_path(path)
            .await
            .with_context(|| format!("failed to open recording file {}", local_path))?;

        // Backblaze B2 S3-compatible PutObject request reference:
        // https://www.backblaze.com/docs/cloud-storage-s3-compatible-apis#put-object
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to upload recording {} to Backblaze B2",
                    recording.id
                )
            })?;

        let duration_sec = recording.duration_sec.unwrap_or(0);

        Ok(UploadResult {
            remote_prefix: object_key,
            size_bytes,
            duration_sec,
        })
    }
}

fn normalize_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{}/", trimmed)
    }
}
