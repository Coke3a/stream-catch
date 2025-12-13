use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::{
    error::{ProvideErrorMetadata, SdkError},
    operation::delete_object::DeleteObjectError,
    operation::put_object::PutObjectError,
    primitives::ByteStream,
};
use uuid::Uuid;

use crate::domain::repositories::storage::CoverStorageClient;

use super::s3::{S3Config, build_s3_client};

#[derive(Debug, Clone)]
pub struct SupabaseStorageConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub prefix: String,
}

pub struct SupabaseStorageClient {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: String,
}

impl SupabaseStorageClient {
    pub async fn new(config: SupabaseStorageConfig) -> Result<Self> {
        let client = build_s3_client(&S3Config {
            endpoint: config.endpoint,
            region: config.region,
            access_key: config.access_key,
            secret_key: config.secret_key,
            force_path_style: true,
            connect_timeout_secs: 10,
            read_timeout_secs: 60,
        })
        .await
        .context("failed to build Supabase s3 client")?;

        Ok(Self {
            client,
            bucket: config.bucket,
            prefix: normalize_prefix(&config.prefix),
        })
    }

    /// Supabase Storage S3-compatible API reference:
    /// https://supabase.com/docs/guides/storage/s3/compatibility
    pub async fn delete_object(&self, object_key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|err| map_delete_object_error(err, &self.bucket, object_key))?;

        Ok(())
    }
}

#[async_trait]
impl CoverStorageClient for SupabaseStorageClient {
    async fn upload_cover(
        &self,
        recording_id: Uuid,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<String> {
        let object_key = format!("{}{}.jpg", self.prefix, recording_id);
        let body = ByteStream::from(bytes);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|err| map_put_object_error(err, &self.bucket, &object_key))?;

        Ok(format!("{}", object_key))
    }

    async fn delete_object(&self, object_key: &str) -> Result<()> {
        SupabaseStorageClient::delete_object(self, object_key).await
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
            "failed to upload cover to Supabase Storage (status {}, code {})",
            status, code
        );

        if !message.is_empty() {
            detail.push_str(&format!(": {}", message));
        }

        detail.push_str(&format!(" [bucket={}, key={}]", bucket, object_key));

        if !body.is_empty() {
            // Keep a short preview of the response body for debugging.
            let preview = body.chars().take(512).collect::<String>();
            detail.push_str(&format!("; body={}", preview));
        }

        return anyhow::anyhow!(detail);
    }

    anyhow::Error::new(err).context("failed to upload cover to Supabase Storage")
}

fn map_delete_object_error(
    err: SdkError<DeleteObjectError>,
    bucket: &str,
    object_key: &str,
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
            "failed to delete Supabase Storage object (status {}, code {})",
            status, code
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

    anyhow::Error::new(err).context("failed to delete object from Supabase Storage")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::storage::CoverStorageClient;
    use anyhow::{Context, Result};
    use std::path::{Path, PathBuf};
    use tokio::fs;
    use uuid::Uuid;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn load_supabase_config_from_env() -> SupabaseStorageConfig {
        dotenvy::dotenv().ok();

        let endpoint = std::env::var("SUPABASE_S3_ENDPOINT").unwrap_or_else(|_| {
            let project_url =
                std::env::var("SUPABASE_PROJECT_URL").expect("SUPABASE_PROJECT_URL is required");
            format!("{}/storage/v1/s3", project_url.trim_end_matches('/'))
        });

        SupabaseStorageConfig {
            endpoint,
            region: std::env::var("SUPABASE_S3_REGION").expect("SUPABASE_S3_REGION is required"),
            bucket: std::env::var("SUPABASE_POSTER_BUCKET")
                .unwrap_or_else(|_| "recording_cover".into()),
            access_key: std::env::var("SUPABASE_S3_ACCESS_KEY_ID")
                .expect("SUPABASE_S3_ACCESS_KEY_ID is required"),
            secret_key: std::env::var("SUPABASE_S3_SECRET_ACCESS_KEY")
                .expect("SUPABASE_S3_SECRET_ACCESS_KEY is required"),
            prefix: std::env::var("SUPABASE_POSTER_PREFIX").unwrap_or_else(|_| "recordings".into()),
        }
    }

    // Manual check: place a JPEG named `test-cover.jpg` in the repo root,
    // export the Supabase S3 credentials, then run:
    // cargo test -p crates supabase_storage::tests::upload_cover_image -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "hits real Supabase Storage and needs local test file + credentials"]
    async fn upload_cover_image() -> Result<()> {
        let image_path = workspace_root().join("test-cover.jpg");
        if !image_path.exists() {
            anyhow::bail!("place `test-cover.jpg` in the project root to run this test");
        }

        let bytes = fs::read(&image_path)
            .await
            .with_context(|| format!("failed to read {:?}", image_path))?;

        let client = SupabaseStorageClient::new(load_supabase_config_from_env()).await?;
        let uploaded_path = client
            .upload_cover(Uuid::new_v4(), bytes, "image/jpeg")
            .await?;
        println!("uploaded cover to {}", uploaded_path);

        Ok(())
    }
}
