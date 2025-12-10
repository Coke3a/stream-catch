use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::{
    error::{ProvideErrorMetadata, SdkError},
    operation::put_object::PutObjectError,
    primitives::ByteStream,
    types::ServerSideEncryption,
};
use mime_guess::MimeGuess;
use tokio::fs;
use uuid::Uuid;

use crate::domain::{
    entities::recordings::RecordingEntity, repositories::storage::StorageClient,
    value_objects::storage::UploadResult,
};

use super::s3::{S3Config, build_s3_client};

#[derive(Clone, Debug)]
pub struct WasabiStorageConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub key_prefix: String,
}

pub struct WasabiStorageClient {
    client: aws_sdk_s3::Client,
    bucket: String,
    key_prefix: String,
}

impl WasabiStorageClient {
    pub async fn new(config: WasabiStorageConfig) -> Result<Self> {
        let WasabiStorageConfig {
            endpoint,
            region,
            bucket,
            access_key_id,
            secret_access_key,
            key_prefix,
        } = config;

        let s3_client = build_s3_client(&S3Config {
            endpoint,
            region,
            access_key: access_key_id,
            secret_key: secret_access_key,
            force_path_style: true,
            connect_timeout_secs: 10,
            read_timeout_secs: 300,
        })
        .await
        .context("failed to build Wasabi s3 client")?;

        let prefix = normalize_prefix(&key_prefix);

        Ok(Self {
            client: s3_client,
            bucket,
            key_prefix: prefix,
        })
    }
}

#[async_trait]
impl StorageClient for WasabiStorageClient {
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

        let object_name = format!("recording-{}_origin.{}", recording.id, extension);
        let object_key = format!("{}{}", self.key_prefix, object_name);

        let content_type = MimeGuess::from_path(path)
            .first_raw()
            .unwrap_or("video/mp4")
            .to_string();

        let body = ByteStream::from_path(path)
            .await
            .with_context(|| format!("failed to open recording file {}", local_path))?;

        // Wasabi PutObject reference:
        // https://wasabi-support.zendesk.com/hc/en-us/articles/115001820872-Amazon-S3-API-Support-and-Compatibility
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .body(body)
            .content_type(content_type)
            .server_side_encryption(ServerSideEncryption::Aes256)
            .send()
            .await
            .map_err(|err| map_put_object_error(err, &self.bucket, &object_key, recording.id))?;

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

fn map_put_object_error(
    err: SdkError<PutObjectError>,
    bucket: &str,
    object_key: &str,
    recording_id: Uuid,
) -> anyhow::Error {
    if let SdkError::ServiceError(service_err) = &err {
        let raw = service_err.raw();
        let status = raw.status().as_u16();
        let code = service_err.err().code().unwrap_or("unknown");
        let message = service_err.err().message().unwrap_or_default();
        let body = raw
            .body()
            .bytes()
            .map(|b| String::from_utf8_lossy(b).trim().to_owned())
            .filter(|b| !b.is_empty())
            .unwrap_or_default();

        let mut detail = format!(
            "failed to upload recording {} to Wasabi (status {}, code {})",
            recording_id, status, code
        );

        if !message.is_empty() {
            detail.push_str(&format!(": {}", message));
        }

        detail.push_str(&format!(" [bucket={}, key={}]", bucket, object_key));

        if !body.is_empty() {
            let preview = body.chars().take(512).collect::<String>();
            detail.push_str(&format!("; body={}", preview));
        }

        return anyhow::anyhow!(detail);
    }

    anyhow::Error::new(err).context(format!(
        "failed to upload recording {} to Wasabi",
        recording_id
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::recordings::RecordingEntity, repositories::storage::StorageClient,
    };
    use anyhow::{Context, Result};
    use chrono::Utc;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn load_wasabi_config_from_env() -> WasabiStorageConfig {
        dotenvy::dotenv().ok();

        WasabiStorageConfig {
            endpoint: std::env::var("VIDEO_STORAGE_S3_ENDPOINT")
                .expect("VIDEO_STORAGE_S3_ENDPOINT is required"),
            region: std::env::var("VIDEO_STORAGE_S3_REGION")
                .expect("VIDEO_STORAGE_S3_REGION is required"),
            bucket: std::env::var("VIDEO_STORAGE_S3_BUCKET")
                .expect("VIDEO_STORAGE_S3_BUCKET is required"),
            access_key_id: std::env::var("VIDEO_STORAGE_S3_ACCESS_KEY_ID")
                .expect("VIDEO_STORAGE_S3_ACCESS_KEY_ID is required"),
            secret_access_key: std::env::var("VIDEO_STORAGE_S3_SECRET_ACCESS_KEY")
                .expect("VIDEO_STORAGE_S3_SECRET_ACCESS_KEY is required"),
            key_prefix: std::env::var("VIDEO_STORAGE_S3_KEY_PREFIX")
                .unwrap_or_else(|_| "recordings".into()),
        }
    }

    fn dummy_recording() -> RecordingEntity {
        let now = Utc::now();
        RecordingEntity {
            id: Uuid::new_v4(),
            live_account_id: Uuid::new_v4(),
            recording_key: None,
            title: Some("manual-upload-check".to_string()),
            started_at: now,
            ended_at: None,
            duration_sec: Some(5),
            size_bytes: None,
            storage_path: None,
            storage_temp_path: None,
            status: "uploading".to_string(),
            poster_storage_path: None,
            created_at: now,
            updated_at: now,
        }
    }

    // Manual check: place an mp4 named `test-recording.mp4` in the repo root,
    // export the VIDEO_STORAGE_S3_* credentials, then run:
    // cargo test -p crates wasabi::tests::upload_mp4_to_wasabi -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "hits real Wasabi and needs local test file + credentials"]
    async fn upload_mp4_to_wasabi() -> Result<()> {
        let mp4_path = workspace_root().join("test-recording.mp4");
        if !mp4_path.exists() {
            anyhow::bail!("place `test-recording.mp4` in the project root to run this test");
        }

        let config = load_wasabi_config_from_env();
        let client = WasabiStorageClient::new(config).await?;

        let path_str = mp4_path
            .to_str()
            .context("failed to convert mp4 path to string")?;
        let recording = dummy_recording();

        let result = client.upload_recording(path_str, &recording).await?;
        println!(
            "uploaded recording to {} ({} bytes). duration: {}",
            result.remote_prefix, result.size_bytes, result.duration_sec
        );

        Ok(())
    }
}
