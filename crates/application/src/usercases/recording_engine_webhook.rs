use anyhow::{Context, Result, bail};
use domain::{
    entities::live_accounts::LiveAccountEntity,
    repositories::recording_engine_webhook::RecordingJobRepository,
    value_objects::{
        enums::{
            live_account_statuses::LiveAccountStatus, platforms::Platform,
            recording_statuses::RecordingStatus,
        },
        recording_engine_webhook::{
            RecordingEngineLiveStartWebhook, RecordingEngineTransmuxFinishWebhook,
        },
        recordings::InsertRecordingModel,
    },
};
use mp4::Mp4Reader;
use reqwest::{Client, header};
use std::{fs::File, io::BufReader, path::{Path, PathBuf}, str::FromStr, sync::Arc, time::Duration};
use tracing::{error, info, warn};
use url::Url;
use uuid::Uuid;

use domain::repositories::job::JobRepository;

pub struct RecordingEngineWebhookUseCase {
    repository: Arc<dyn RecordingJobRepository + Send + Sync>,
    job_repository: Arc<dyn JobRepository + Send + Sync>,
    storage_config: SupabaseStorageConfig,
    http_client: Client,
    allowed_recording_base: PathBuf,
}

#[derive(Clone)]
pub struct SupabaseStorageConfig {
    pub project_url: String,
    pub service_key: String,
    pub poster_bucket: String,
}

impl RecordingEngineWebhookUseCase {
    pub fn new(
        repository: Arc<dyn RecordingJobRepository + Send + Sync>,
        job_repository: Arc<dyn JobRepository + Send + Sync>,
        storage_config: SupabaseStorageConfig,
        allowed_recording_base: PathBuf,
    ) -> Self {
        Self {
            repository,
            job_repository,
            storage_config,
            http_client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("failed to build http client"),
            allowed_recording_base,
        }
    }

    pub async fn get_unsynced_live_accounts(&self) -> Result<Vec<LiveAccountEntity>> {
        let accounts = self.repository.find_unsynced_live_accounts().await?;
        info!(count = accounts.len(), "found unsynced live accounts");
        Ok(accounts)
    }

    pub async fn update_live_account_status(
        &self,
        id: Uuid,
        status: LiveAccountStatus,
    ) -> Result<Uuid> {
        info!(live_account_id = %id, %status, "updating live account status");
        self.repository.update_live_account_status(id, status).await
    }

    pub async fn handle_live_start(
        &self,
        payload: RecordingEngineLiveStartWebhook,
    ) -> Result<Uuid> {
        info!(payload_id = %payload.id, "handling live_start webhook");
        let data = payload.data;
        let platform = self.parse_platform(data.platform)?;
        let channel = data
            .channel
            .clone()
            .ok_or_else(|| anyhow::anyhow!("channel is required"))?;

        let live_account = self
            .repository
            .find_live_account_by_platform_and_account_id(platform.to_string(), channel.clone())
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Live account not found for platform {} and channel {}",
                    platform,
                    channel
                )
            })?;

        info!(platform = %platform, channel, live_account_id = %live_account.id, "live_start: found live account");
        let live_info = data
            .live_info
            .ok_or_else(|| anyhow::anyhow!("live_info is required"))?;
        let title = live_info.title.clone();
        let cover = live_info
            .cover
            .ok_or_else(|| anyhow::anyhow!("cover is required for poster upload"))?;

        let poster_storage_path = self.upload_cover(&cover).await?;

        let insert_model = InsertRecordingModel {
            live_account_id: live_account.id,
            poster_storage_path: Some(poster_storage_path),
            title,
        };

        let insert_entity = insert_model.to_entity();
        let recording_id = self.repository.insert(insert_entity).await?;
        info!(%recording_id, "live_start: recording inserted");
        Ok(recording_id)
    }

    pub async fn handle_transmux_finish(
        &self,
        payload: RecordingEngineTransmuxFinishWebhook,
    ) -> Result<Uuid> {
        info!(payload_id = %payload.id, "handling video_transmux_finish webhook");
        let data = payload.data;
        let platform = self.parse_platform(data.platform)?;
        let platform_string = platform.to_string();
        let channel = data
            .channel
            .clone()
            .ok_or_else(|| anyhow::anyhow!("channel is required"))?;

        let storage_path_raw = data
            .output
            .clone()
            .ok_or_else(|| anyhow::anyhow!("output storage path is required"))?;
        let storage_path = self.validate_local_path(&storage_path_raw)?;

        let recording_id = if let Some(recording) = self.repository
            .find_recording_by_live_account_and_status(
                platform_string.clone(),
                channel.clone(),
                RecordingStatus::LiveRecording,
            )
            .await?
        {
            recording.id
        } else {
            let live_account = self
                .repository
                .find_live_account_by_platform_and_account_id(
                    platform_string.clone(),
                    channel.clone(),
                )
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Live account not found for platform {} and channel {}",
                        platform,
                        channel
                    )
                })?;

            let insert_model = InsertRecordingModel {
                live_account_id: live_account.id,
                poster_storage_path: None,
                title: None,
            };

            let new_id = self.repository.insert(insert_model.to_entity()).await?;
            info!(%new_id, "transmux_finish: inserted placeholder recording");
            new_id
        };

        let duration_sec = if Self::is_mp4_path(&storage_path) {
            match Self::read_mp4_duration_seconds(storage_path.clone()).await {
                Ok(duration) => Some(duration),
                Err(err) => {
                    error!(
                        path = %storage_path.display(),
                        "failed to read duration for mp4 output: {:?}",
                        err
                    );
                    None
                }
            }
        } else {
            warn!(path = %storage_path.display(), "transmux output is not an mp4 file");
            None
        };

        // Update recording status to WaitingUpload and store local path
        let path_str = storage_path.to_string_lossy().into_owned();
        let updated_recording_id = self
            .repository
            .update_live_transmux_finish(recording_id, path_str.clone(), duration_sec)
            .await?;

        // Enqueue upload job
        self.job_repository
            .enqueue_recording_upload_job(updated_recording_id, path_str)
            .await?;

        info!(%updated_recording_id, "transmux_finish: enqueued upload job and updated recording");

        Ok(updated_recording_id)
    }

    pub async fn handle_uploading_status(
        &self,
        platform: Option<String>,
        channel: Option<String>,
    ) -> Result<Uuid> {
        let parsed_platform = self.parse_platform(platform)?;
        let channel = channel.ok_or_else(|| anyhow::anyhow!("channel is required"))?;

        let recording = self
            .repository
            .find_recording_by_live_account_and_status(
                parsed_platform.to_string(),
                channel.clone(),
                RecordingStatus::WaitingUpload,
            )
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Recording not found for platform {} and channel {} in status waiting_upload",
                    parsed_platform,
                    channel
                )
            })?;

        info!(recording_id = %recording.id, "marking recording as uploading");
        self.repository.update_file_uploading(recording.id).await
    }

    fn parse_platform(&self, platform: Option<String>) -> Result<Platform> {
        let platform_str = platform.ok_or_else(|| anyhow::anyhow!("platform is required"))?;
        Platform::from_str(&platform_str)
            .map_err(|_| anyhow::anyhow!("Unsupported platform: {}", platform_str))
    }

    fn is_mp4_path(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("mp4"))
            .unwrap_or(false)
    }

    async fn read_mp4_duration_seconds(path: PathBuf) -> Result<i32> {
        tokio::task::spawn_blocking(move || {
            let file = File::open(&path)?;
            let size = file.metadata()?.len();
            let reader = BufReader::new(file);
            let mp4 = Mp4Reader::read_header(reader, size)?;
            let duration = mp4.duration().as_secs_f64().round() as i64;

            i32::try_from(duration).context("mp4 duration seconds exceed i32")
        })
        .await
        .context("failed to join duration reader task")?
    }

    fn validate_local_path(&self, path: &str) -> Result<PathBuf> {
        let base = self
            .allowed_recording_base
            .canonicalize()
            .context("failed to canonicalize allowed recording base")?;

        let candidate = PathBuf::from(path);
        let canonical = candidate
            .canonicalize()
            .with_context(|| format!("failed to canonicalize recording path: {path}"))?;

        if !canonical.starts_with(&base) {
            bail!("recording path is outside the allowed directory");
        }

        Ok(canonical)
    }

    async fn upload_cover(&self, cover_url: &str) -> Result<String> {
        let trimmed_cover = cover_url.trim();
        if trimmed_cover.is_empty() {
            bail!("cover url cannot be empty");
        }

        let response = self
            .http_client
            .get(trimmed_cover)
            .send()
            .await
            .context("failed to download cover image")?;

        if !response.status().is_success() {
            bail!(
                "failed to download cover image, status: {}",
                response.status()
            );
        }

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let bytes = response
            .bytes()
            .await
            .context("failed to read cover bytes")?;

        let extension = Url::parse(trimmed_cover)
            .ok()
            .and_then(|url| {
                url.path_segments()
                    .and_then(|segments| segments.last().map(String::from))
            })
            .and_then(|filename| filename.split('.').last().map(String::from))
            .filter(|ext| !ext.is_empty())
            .unwrap_or_else(|| "jpg".to_string());

        let object_name = format!("poster-{}.{}", Uuid::new_v4(), extension);
        let object_path = format!("recordings/{}", object_name);

        let upload_url = format!(
            "{}/storage/v1/object/{}/{}",
            self.storage_config.project_url.trim_end_matches('/'),
            self.storage_config.poster_bucket,
            object_path
        );

        let upload_response = self
            .http_client
            .post(upload_url)
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.storage_config.service_key),
            )
            .header("apikey", self.storage_config.service_key.clone())
            .header("x-upsert", "true")
            .header(header::CONTENT_TYPE, content_type)
            .body(bytes)
            .send()
            .await
            .context("failed to upload poster to supabase storage")?;

        if !upload_response.status().is_success() {
            bail!(
                "supabase storage upload failed with status: {}",
                upload_response.status()
            );
        }

        let stored_path = format!("{}/{}", self.storage_config.poster_bucket, object_path);
        info!(object = stored_path, "uploaded cover image to storage");
        Ok(stored_path)
    }
}
