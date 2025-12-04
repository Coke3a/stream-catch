use anyhow::{Context, Result, bail};
use chrono::Utc;
use domain::{
    entities::{
        live_accounts::LiveAccountEntity,
        recordings::RecordingTransmuxUpdateEntity,
    },
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
use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tracing::{error, info, warn};
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

        let insert_model = InsertRecordingModel {
            live_account_id: live_account.id,
            poster_storage_path: None,
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

        let poster_storage_path = self
            .find_and_upload_cover_from_output(&storage_path)
            .await?;

        let recording_id = if let Some(recording) = self
            .repository
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
        let changeset = RecordingTransmuxUpdateEntity {
            storage_path: Some(path_str.clone()),
            duration_sec,
            status: RecordingStatus::WaitingUpload.to_string(),
            updated_at: Utc::now(),
            poster_storage_path: poster_storage_path.map(Some),
        };

        let updated_recording_id = self
            .repository
            .update_live_transmux_finish(
                recording_id,
                changeset,
            )
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

    async fn find_and_upload_cover_from_output(
        &self,
        video_output_path: &Path,
    ) -> Result<Option<String>> {
        if !video_output_path.exists() {
            warn!(path = %video_output_path.display(), "transmux output path does not exist");
            return Ok(None);
        }

        let candidate = self.find_existing_cover_path(video_output_path).await?;
        let Some(cover_path) = candidate else {
            return Ok(None);
        };

        let stored_path = self.upload_cover_from_file(&cover_path).await?;
        Ok(Some(stored_path))
    }

    async fn find_existing_cover_path(&self, video_output_path: &Path) -> Result<Option<PathBuf>> {
        let extensions = ["jpg", "jpeg", "png", "webp"];
        for ext in extensions {
            let candidate = video_output_path.with_extension(ext);
            if candidate.exists() {
                let canonical = self.validate_local_path(&candidate.to_string_lossy())?;
                return Ok(Some(canonical));
            }
        }

        Ok(None)
    }

    async fn upload_cover_from_file(&self, cover_path: &Path) -> Result<String> {
        let bytes = fs::read(cover_path)
            .with_context(|| format!("failed to read cover file: {}", cover_path.display()))?;

        let content_type = cover_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_ascii_lowercase().as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            })
            .unwrap_or("application/octet-stream");

        let extension = cover_path
            .extension()
            .and_then(|ext| ext.to_str())
            .filter(|ext| !ext.is_empty())
            .unwrap_or("jpg");

        let object_name = format!("poster-{}.{}", Uuid::new_v4(), extension);
        let object_path = format!("recordings/{}", object_name);

        let upload_url = format!(
            "{}/storage/v1/object/{}/{}",
            self.storage_config.project_url.trim_end_matches('/'),
            self.storage_config.poster_bucket,
            object_path
        );

        // Supabase Storage upload API: https://supabase.com/docs/guides/storage/api/upload
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
