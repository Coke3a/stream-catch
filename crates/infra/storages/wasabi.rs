use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::{
    error::{ProvideErrorMetadata, SdkError},
    operation::abort_multipart_upload::AbortMultipartUploadError,
    operation::complete_multipart_upload::CompleteMultipartUploadError,
    operation::create_multipart_upload::CreateMultipartUploadError,
    operation::delete_object::DeleteObjectError,
    operation::put_object::PutObjectError,
    operation::upload_part::UploadPartError,
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart, ServerSideEncryption},
};
use bytes::Bytes;
use mime_guess::MimeGuess;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use tracing::Instrument;
use uuid::Uuid;

use crate::domain::{
    entities::recordings::RecordingEntity, repositories::storage::StorageClient,
    value_objects::storage::UploadResult,
};

use super::s3::{S3Config, StorageUploadError, build_s3_client, is_retryable_s3_error};

#[derive(Clone, Debug)]
pub struct WasabiStorageConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub key_prefix: String,
    pub multipart: WasabiMultipartConfig,
}

#[derive(Clone, Debug)]
pub struct WasabiMultipartConfig {
    pub enabled: bool,
    pub threshold_bytes: u64,
    pub part_size_bytes: u64,
    pub per_file_concurrency: usize,
    pub global_concurrency: usize,
    pub max_retries: usize,
    pub backoff_base_ms: u64,
    pub backoff_max_ms: u64,
}

const MIN_PART_SIZE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_PART_SIZE_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const MAX_MULTIPART_PARTS: u64 = 10_000;

pub struct WasabiStorageClient {
    client: aws_sdk_s3::Client,
    bucket: String,
    key_prefix: String,
    multipart: WasabiMultipartConfig,
    global_part_limiter: Arc<Semaphore>,
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
            multipart,
        } = config;

        validate_multipart_config(&multipart)?;

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
            global_part_limiter: Arc::new(Semaphore::new(multipart.global_concurrency)),
            multipart,
        })
    }

    /// Wasabi DeleteObject reference:
    /// https://wasabi-support.zendesk.com/hc/en-us/articles/115001820872-Amazon-S3-API-Support-and-Compatibility
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

    async fn upload_recording_single(
        &self,
        path: &Path,
        object_key: &str,
        content_type: &str,
        recording_id: Uuid,
    ) -> Result<()> {
        let body = ByteStream::from_path(path)
            .await
            .map_err(|err| {
                StorageUploadError::non_retryable_with_source(
                    format!("failed to open recording file {}", path.display()),
                    err.into(),
                )
            })?;

        // Wasabi PutObject reference:
        // https://wasabi-support.zendesk.com/hc/en-us/articles/115001820872-Amazon-S3-API-Support-and-Compatibility
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(object_key)
            .body(body)
            .content_type(content_type)
            .server_side_encryption(ServerSideEncryption::Aes256)
            .send()
            .await
            .map_err(|err| {
                let retryable = is_retryable_s3_error(&err);
                let mapped = map_put_object_error(err, &self.bucket, object_key, recording_id);
                if retryable {
                    StorageUploadError::retryable_with_source(mapped.to_string(), mapped)
                } else {
                    StorageUploadError::non_retryable_with_source(mapped.to_string(), mapped)
                }
            })?;

        info!(
            recording_id = %recording_id,
            bucket = %self.bucket,
            key = %object_key,
            "wasabi upload completed"
        );

        Ok(())
    }

    async fn upload_recording_multipart(
        &self,
        path: &Path,
        object_key: &str,
        content_type: &str,
        recording_id: Uuid,
        size_bytes: u64,
    ) -> Result<()> {
        let part_size = self.resolve_part_size(size_bytes)?;
        let total_parts = calculate_total_parts(size_bytes, part_size);

        if part_size != self.multipart.part_size_bytes {
            info!(
                recording_id = %recording_id,
                bucket = %self.bucket,
                key = %object_key,
                part_size_bytes = part_size,
                configured_part_size_bytes = self.multipart.part_size_bytes,
                "wasabi multipart part size adjusted to stay within limits"
            );
        }

        // Multipart flow: create upload, stream parts, then complete (abort on failure).
        let upload_id = self
            .create_multipart_upload(object_key, content_type, recording_id)
            .await?;

        info!(
            recording_id = %recording_id,
            bucket = %self.bucket,
            key = %object_key,
            upload_id = %upload_id,
            part_size_bytes = part_size,
            total_parts,
            "wasabi multipart upload initiated"
        );

        let span = tracing::Span::current();
        let parts_result = self
            .upload_parts(
                path,
                object_key,
                &upload_id,
                recording_id,
                size_bytes,
                part_size,
                total_parts,
                span,
            )
            .await;

        let mut completed_parts = match parts_result {
            Ok(parts) => parts,
            Err(err) => {
                warn!(
                    recording_id = %recording_id,
                    bucket = %self.bucket,
                    key = %object_key,
                    upload_id = %upload_id,
                    "wasabi multipart upload failed; aborting"
                );
                if let Err(abort_err) = self
                    .abort_multipart_upload(object_key, &upload_id, recording_id)
                    .await
                {
                    error!(
                        recording_id = %recording_id,
                        bucket = %self.bucket,
                        key = %object_key,
                        upload_id = %upload_id,
                        error = %abort_err,
                        "wasabi multipart abort failed"
                    );
                }
                return Err(err);
            }
        };

        completed_parts.sort_by_key(|part| part.part_number().unwrap_or_default());

        if let Err(err) = self
            .complete_multipart_upload(object_key, &upload_id, completed_parts, recording_id)
            .await
        {
            warn!(
                recording_id = %recording_id,
                bucket = %self.bucket,
                key = %object_key,
                upload_id = %upload_id,
                "wasabi multipart completion failed; aborting"
            );
            if let Err(abort_err) = self
                .abort_multipart_upload(object_key, &upload_id, recording_id)
                .await
            {
                error!(
                    recording_id = %recording_id,
                    bucket = %self.bucket,
                    key = %object_key,
                    upload_id = %upload_id,
                    error = %abort_err,
                    "wasabi multipart abort failed"
                );
            }
            return Err(err);
        }

        info!(
            recording_id = %recording_id,
            bucket = %self.bucket,
            key = %object_key,
            upload_id = %upload_id,
            "wasabi multipart upload completed"
        );

        Ok(())
    }

    async fn create_multipart_upload(
        &self,
        object_key: &str,
        content_type: &str,
        recording_id: Uuid,
    ) -> Result<String> {
        let response = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(object_key)
            .content_type(content_type)
            .server_side_encryption(ServerSideEncryption::Aes256)
            .send()
            .await
            .map_err(|err| {
                let retryable = is_retryable_s3_error(&err);
                let mapped =
                    map_create_multipart_upload_error(err, &self.bucket, object_key, recording_id);
                if retryable {
                    StorageUploadError::retryable_with_source(mapped.to_string(), mapped)
                } else {
                    StorageUploadError::non_retryable_with_source(mapped.to_string(), mapped)
                }
            })?;

        let upload_id = response.upload_id().ok_or_else(|| {
            StorageUploadError::retryable(format!(
                "missing upload id for multipart recording {}",
                recording_id
            ))
        })?;

        Ok(upload_id.to_string())
    }

    async fn complete_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
        completed_parts: Vec<CompletedPart>,
        recording_id: Uuid,
    ) -> Result<()> {
        let upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(object_key)
            .upload_id(upload_id)
            .multipart_upload(upload)
            .send()
            .await
            .map_err(|err| {
                let retryable = is_retryable_s3_error(&err);
                let mapped = map_complete_multipart_upload_error(
                    err,
                    &self.bucket,
                    object_key,
                    upload_id,
                    recording_id,
                );
                if retryable {
                    StorageUploadError::retryable_with_source(mapped.to_string(), mapped)
                } else {
                    StorageUploadError::non_retryable_with_source(mapped.to_string(), mapped)
                }
            })?;

        Ok(())
    }

    async fn abort_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
        recording_id: Uuid,
    ) -> Result<()> {
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(object_key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|err| {
                let mapped = map_abort_multipart_upload_error(
                    err,
                    &self.bucket,
                    object_key,
                    upload_id,
                    recording_id,
                );
                StorageUploadError::retryable_with_source(mapped.to_string(), mapped)
            })?;

        warn!(
            recording_id = %recording_id,
            bucket = %self.bucket,
            key = %object_key,
            upload_id = %upload_id,
            "wasabi multipart upload aborted"
        );

        Ok(())
    }

    async fn upload_parts(
        &self,
        path: &Path,
        object_key: &str,
        upload_id: &str,
        recording_id: Uuid,
        size_bytes: u64,
        part_size: u64,
        total_parts: u64,
        span: tracing::Span,
    ) -> Result<Vec<CompletedPart>> {
        let mut file = fs::File::open(path)
            .await
            .map_err(|err| {
                StorageUploadError::non_retryable_with_source(
                    format!("failed to open recording file {}", path.display()),
                    err.into(),
                )
            })?;
        let per_file_limiter = Arc::new(Semaphore::new(self.multipart.per_file_concurrency));

        let mut join_set = JoinSet::new();
        let mut completed_parts = Vec::with_capacity(total_parts as usize);
        let mut offset = 0_u64;
        let mut part_number: u32 = 1;

        while offset < size_bytes {
            // Acquire permits before reading to bound in-memory buffering.
            let per_file_permit = match per_file_limiter.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(err) => {
                    abort_inflight_parts(&mut join_set).await;
                    return Err(StorageUploadError::non_retryable_with_source(
                        "multipart per-file limiter closed",
                        err.into(),
                    ));
                }
            };
            let global_permit = match self.global_part_limiter.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(err) => {
                    abort_inflight_parts(&mut join_set).await;
                    return Err(StorageUploadError::non_retryable_with_source(
                        "multipart global limiter closed",
                        err.into(),
                    ));
                }
            };

            let remaining = size_bytes.saturating_sub(offset);
            let chunk_size = remaining.min(part_size);
            let chunk_size_usize = match usize::try_from(chunk_size) {
                Ok(size) => size,
                Err(err) => {
                    abort_inflight_parts(&mut join_set).await;
                    return Err(StorageUploadError::non_retryable_with_source(
                        "multipart part size exceeds addressable memory",
                        err.into(),
                    ));
                }
            };
            let mut buffer = vec![0u8; chunk_size_usize];
            if let Err(err) = file.read_exact(&mut buffer).await {
                abort_inflight_parts(&mut join_set).await;
                return Err(StorageUploadError::non_retryable_with_source(
                    format!("failed to read multipart chunk {}", part_number),
                    err.into(),
                ));
            }

            let bytes = Bytes::from(buffer);
            let client = self.client.clone();
            let bucket = self.bucket.clone();
            let key = object_key.to_string();
            let upload_id = upload_id.to_string();
            let config = self.multipart.clone();
            let task = async move {
                let _per_file_permit = per_file_permit;
                let _global_permit = global_permit;
                upload_part_with_retry(
                    client,
                    bucket,
                    key,
                    upload_id,
                    part_number,
                    bytes,
                    recording_id,
                    config,
                )
                .await
            };

            join_set.spawn(task.instrument(span.clone()));

            offset = offset.saturating_add(chunk_size);
            part_number += 1;

            if join_set.len() >= self.multipart.per_file_concurrency {
                if let Some(result) = join_set.join_next().await {
                    match result {
                        Ok(Ok(part)) => completed_parts.push(part),
                        Ok(Err(err)) => {
                            abort_inflight_parts(&mut join_set).await;
                            return Err(err);
                        }
                        Err(err) => {
                            abort_inflight_parts(&mut join_set).await;
                            return Err(StorageUploadError::retryable_with_source(
                                "multipart upload task failed",
                                err.into(),
                            ));
                        }
                    }
                }
            }
        }

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(part)) => completed_parts.push(part),
                Ok(Err(err)) => {
                    abort_inflight_parts(&mut join_set).await;
                    return Err(err);
                }
                Err(err) => {
                    abort_inflight_parts(&mut join_set).await;
                    return Err(StorageUploadError::retryable_with_source(
                        "multipart upload task failed",
                        err.into(),
                    ));
                }
            }
        }

        Ok(completed_parts)
    }

    fn should_use_multipart(&self, size_bytes: u64) -> bool {
        if !self.multipart.enabled || size_bytes == 0 {
            return false;
        }
        if self.multipart.threshold_bytes == 0 {
            return true;
        }
        size_bytes >= self.multipart.threshold_bytes
    }

    fn resolve_part_size(&self, size_bytes: u64) -> Result<u64> {
        if self.multipart.part_size_bytes < MIN_PART_SIZE_BYTES
            || self.multipart.part_size_bytes > MAX_PART_SIZE_BYTES
        {
            return Err(StorageUploadError::non_retryable(format!(
                "multipart part size must be between {} and {} bytes",
                MIN_PART_SIZE_BYTES, MAX_PART_SIZE_BYTES
            )));
        }

        if size_bytes == 0 {
            return Ok(self.multipart.part_size_bytes);
        }

        let min_part_size = ceil_div(size_bytes, MAX_MULTIPART_PARTS);
        let mut part_size = self.multipart.part_size_bytes.max(min_part_size);
        if part_size < MIN_PART_SIZE_BYTES {
            part_size = MIN_PART_SIZE_BYTES;
        }

        if part_size > MAX_PART_SIZE_BYTES {
            return Err(StorageUploadError::non_retryable(format!(
                "multipart part size {} exceeds maximum {}",
                part_size, MAX_PART_SIZE_BYTES
            )));
        }

        Ok(part_size)
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
            return Err(StorageUploadError::non_retryable(format!(
                "local file does not exist: {}",
                local_path
            )));
        }

        let metadata = fs::metadata(path)
            .await
            .map_err(|err| {
                StorageUploadError::non_retryable_with_source(
                    format!("failed to read metadata for {}", local_path),
                    err.into(),
                )
            })?;
        let size_bytes = metadata.len();
        let size_bytes_i64 = i64::try_from(size_bytes).map_err(|err| {
            StorageUploadError::non_retryable_with_source(
                "recording size is too large",
                err.into(),
            )
        })?;

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

        let use_multipart = self.should_use_multipart(size_bytes);
        info!(
            recording_id = %recording.id,
            bucket = %self.bucket,
            key = %object_key,
            size_bytes,
            multipart = use_multipart,
            "wasabi upload started"
        );

        if use_multipart {
            self.upload_recording_multipart(
                path,
                &object_key,
                &content_type,
                recording.id,
                size_bytes,
            )
            .await?;
        } else {
            self.upload_recording_single(
                path,
                &object_key,
                &content_type,
                recording.id,
            )
            .await?;
        }

        let duration_sec = recording.duration_sec.unwrap_or(0);

        Ok(UploadResult {
            remote_prefix: object_key,
            size_bytes: size_bytes_i64,
            duration_sec,
        })
    }

    async fn delete_object(&self, object_key: &str) -> Result<()> {
        WasabiStorageClient::delete_object(self, object_key).await
    }
}

async fn upload_part_with_retry(
    client: aws_sdk_s3::Client,
    bucket: String,
    key: String,
    upload_id: String,
    part_number: u32,
    bytes: Bytes,
    recording_id: Uuid,
    config: WasabiMultipartConfig,
) -> Result<CompletedPart> {
    let bytes_len = bytes.len() as u64;
    let content_length = i64::try_from(bytes_len).map_err(|err| {
        StorageUploadError::non_retryable_with_source(
            "multipart part size is too large",
            err.into(),
        )
    })?;
    let max_attempts = config.max_retries.saturating_add(1).max(1);
    let mut attempt = 0;

    // Retry transient errors with exponential backoff; non-retryable errors stop immediately.
    loop {
        attempt += 1;
        let start = Instant::now();
        let body = ByteStream::from(bytes.clone());

        let result = client
            .upload_part()
            .bucket(&bucket)
            .key(&key)
            .upload_id(&upload_id)
            .part_number(part_number as i32)
            .content_length(content_length)
            .body(body)
            .send()
            .await;

        match result {
            Ok(output) => {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                if let Some(e_tag) = output.e_tag() {
                    info!(
                        recording_id = %recording_id,
                        bucket = %bucket,
                        key = %key,
                        upload_id = %upload_id,
                        part_number,
                        attempt,
                        bytes_uploaded = bytes_len,
                        elapsed_ms,
                        "wasabi multipart part uploaded"
                    );
                    return Ok(CompletedPart::builder()
                        .e_tag(e_tag)
                        .part_number(part_number as i32)
                        .build());
                }

                if attempt >= max_attempts {
                    return Err(StorageUploadError::retryable(format!(
                        "missing ETag for multipart part {}",
                        part_number
                    )));
                }

                let backoff = calculate_backoff(attempt, &config);
                warn!(
                    recording_id = %recording_id,
                    bucket = %bucket,
                    key = %key,
                    upload_id = %upload_id,
                    part_number,
                    attempt,
                    bytes_uploaded = bytes_len,
                    elapsed_ms,
                    backoff_ms = backoff.as_millis() as u64,
                    "wasabi multipart part missing ETag; retrying"
                );
                tokio::time::sleep(backoff).await;
            }
            Err(err) => {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                let retryable = is_retryable_s3_error(&err);
                if retryable && attempt < max_attempts {
                    let backoff = calculate_backoff(attempt, &config);
                    warn!(
                        recording_id = %recording_id,
                        bucket = %bucket,
                        key = %key,
                        upload_id = %upload_id,
                        part_number,
                        attempt,
                        bytes_uploaded = bytes_len,
                        elapsed_ms,
                        backoff_ms = backoff.as_millis() as u64,
                        "wasabi multipart part upload failed; retrying"
                    );
                    tokio::time::sleep(backoff).await;
                    continue;
                }

                let mapped = map_upload_part_error(
                    err,
                    &bucket,
                    &key,
                    &upload_id,
                    part_number,
                    recording_id,
                );
                let message = mapped.to_string();
                return Err(if retryable {
                    StorageUploadError::retryable_with_source(message, mapped)
                } else {
                    StorageUploadError::non_retryable_with_source(message, mapped)
                });
            }
        }
    }
}

async fn abort_inflight_parts(join_set: &mut JoinSet<Result<CompletedPart>>) {
    join_set.abort_all();
    while join_set.join_next().await.is_some() {}
}

fn validate_multipart_config(config: &WasabiMultipartConfig) -> Result<()> {
    if config.per_file_concurrency == 0 {
        return Err(StorageUploadError::non_retryable(
            "multipart per-file concurrency must be >= 1",
        ));
    }
    if config.global_concurrency == 0 {
        return Err(StorageUploadError::non_retryable(
            "multipart global concurrency must be >= 1",
        ));
    }
    if config.part_size_bytes < MIN_PART_SIZE_BYTES || config.part_size_bytes > MAX_PART_SIZE_BYTES {
        return Err(StorageUploadError::non_retryable(format!(
            "multipart part size must be between {} and {} bytes",
            MIN_PART_SIZE_BYTES, MAX_PART_SIZE_BYTES
        )));
    }
    Ok(())
}

fn calculate_backoff(attempt: usize, config: &WasabiMultipartConfig) -> Duration {
    let exponent = attempt.saturating_sub(1) as u32;
    let multiplier = 2u64.saturating_pow(exponent);
    let base = config.backoff_base_ms.saturating_mul(multiplier);
    let capped = base.min(config.backoff_max_ms);
    Duration::from_millis(capped)
}

fn calculate_total_parts(size_bytes: u64, part_size: u64) -> u64 {
    if size_bytes == 0 || part_size == 0 {
        0
    } else {
        ceil_div(size_bytes, part_size)
    }
}

fn ceil_div(numerator: u64, denominator: u64) -> u64 {
    if numerator == 0 {
        0
    } else {
        (numerator + denominator - 1) / denominator
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

fn map_create_multipart_upload_error(
    err: SdkError<CreateMultipartUploadError>,
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
            "failed to create multipart upload for recording {} to Wasabi (status {}, code {})",
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
        "failed to create multipart upload for recording {}",
        recording_id
    ))
}

fn map_upload_part_error(
    err: SdkError<UploadPartError>,
    bucket: &str,
    object_key: &str,
    upload_id: &str,
    part_number: u32,
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
            "failed to upload part {} for recording {} to Wasabi (status {}, code {})",
            part_number, recording_id, status, code
        );

        if !message.is_empty() {
            detail.push_str(&format!(": {}", message));
        }

        detail.push_str(&format!(
            " [bucket={}, key={}, upload_id={}]",
            bucket, object_key, upload_id
        ));

        if !body.is_empty() {
            let preview = body.chars().take(512).collect::<String>();
            detail.push_str(&format!("; body={}", preview));
        }

        return anyhow::anyhow!(detail);
    }

    anyhow::Error::new(err).context(format!(
        "failed to upload part {} for recording {} to Wasabi",
        part_number, recording_id
    ))
}

fn map_complete_multipart_upload_error(
    err: SdkError<CompleteMultipartUploadError>,
    bucket: &str,
    object_key: &str,
    upload_id: &str,
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
            "failed to complete multipart upload for recording {} to Wasabi (status {}, code {})",
            recording_id, status, code
        );

        if !message.is_empty() {
            detail.push_str(&format!(": {}", message));
        }

        detail.push_str(&format!(
            " [bucket={}, key={}, upload_id={}]",
            bucket, object_key, upload_id
        ));

        if !body.is_empty() {
            let preview = body.chars().take(512).collect::<String>();
            detail.push_str(&format!("; body={}", preview));
        }

        return anyhow::anyhow!(detail);
    }

    anyhow::Error::new(err).context(format!(
        "failed to complete multipart upload for recording {}",
        recording_id
    ))
}

fn map_abort_multipart_upload_error(
    err: SdkError<AbortMultipartUploadError>,
    bucket: &str,
    object_key: &str,
    upload_id: &str,
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
            "failed to abort multipart upload for recording {} to Wasabi (status {}, code {})",
            recording_id, status, code
        );

        if !message.is_empty() {
            detail.push_str(&format!(": {}", message));
        }

        detail.push_str(&format!(
            " [bucket={}, key={}, upload_id={}]",
            bucket, object_key, upload_id
        ));

        if !body.is_empty() {
            let preview = body.chars().take(512).collect::<String>();
            detail.push_str(&format!("; body={}", preview));
        }

        return anyhow::anyhow!(detail);
    }

    anyhow::Error::new(err).context(format!(
        "failed to abort multipart upload for recording {}",
        recording_id
    ))
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
            "failed to delete Wasabi object (status {}, code {})",
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

    anyhow::Error::new(err).context("failed to delete Wasabi object")
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
            multipart: WasabiMultipartConfig {
                enabled: std::env::var("VIDEO_STORAGE_MULTIPART_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .expect("VIDEO_STORAGE_MULTIPART_ENABLED is invalid"),
                threshold_bytes: std::env::var("VIDEO_STORAGE_MULTIPART_THRESHOLD_BYTES")
                    .unwrap_or_else(|_| "268435456".to_string())
                    .parse()
                    .expect("VIDEO_STORAGE_MULTIPART_THRESHOLD_BYTES is invalid"),
                part_size_bytes: std::env::var("VIDEO_STORAGE_MULTIPART_PART_SIZE_MB")
                    .unwrap_or_else(|_| "128".to_string())
                    .parse::<u64>()
                    .expect("VIDEO_STORAGE_MULTIPART_PART_SIZE_MB is invalid")
                    * 1024
                    * 1024,
                per_file_concurrency: std::env::var("VIDEO_STORAGE_MULTIPART_PER_FILE_CONCURRENCY")
                    .unwrap_or_else(|_| "4".to_string())
                    .parse()
                    .expect("VIDEO_STORAGE_MULTIPART_PER_FILE_CONCURRENCY is invalid"),
                global_concurrency: std::env::var("VIDEO_STORAGE_MULTIPART_GLOBAL_CONCURRENCY")
                    .unwrap_or_else(|_| "8".to_string())
                    .parse()
                    .expect("VIDEO_STORAGE_MULTIPART_GLOBAL_CONCURRENCY is invalid"),
                max_retries: std::env::var("VIDEO_STORAGE_MULTIPART_MAX_RETRIES")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .expect("VIDEO_STORAGE_MULTIPART_MAX_RETRIES is invalid"),
                backoff_base_ms: std::env::var("VIDEO_STORAGE_MULTIPART_BACKOFF_BASE_MS")
                    .unwrap_or_else(|_| "500".to_string())
                    .parse()
                    .expect("VIDEO_STORAGE_MULTIPART_BACKOFF_BASE_MS is invalid"),
                backoff_max_ms: std::env::var("VIDEO_STORAGE_MULTIPART_BACKOFF_MAX_MS")
                    .unwrap_or_else(|_| "15000".to_string())
                    .parse()
                    .expect("VIDEO_STORAGE_MULTIPART_BACKOFF_MAX_MS is invalid"),
            },
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
