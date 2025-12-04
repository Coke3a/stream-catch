use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
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
        })
        .await
        .context("failed to build Supabase s3 client")?;

        Ok(Self {
            client,
            bucket: config.bucket,
            prefix: normalize_prefix(&config.prefix),
        })
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
            .context("failed to upload cover to Supabase Storage")?;

        Ok(format!("{}/{}", self.bucket, object_key))
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
