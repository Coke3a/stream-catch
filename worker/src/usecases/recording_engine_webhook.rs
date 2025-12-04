use anyhow::{Context, Result, bail};
use chrono::Utc;
use crates::domain;
use domain::{
    entities::recordings::RecordingTransmuxUpdateEntity,
    repositories::recording_engine_webhook::RecordingEngineWebhookRepository,
    value_objects::{
        enums::{platforms::Platform, recording_statuses::RecordingStatus},
        recording_engine_webhook::{
            RecordingEngineLiveStartWebhook, RecordingEngineTransmuxFinishWebhook,
        },
        recordings::InsertRecordingModel,
    },
};
use mp4::Mp4Reader;
use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

use domain::repositories::job::JobRepository;
use domain::repositories::storage::CoverStorageClient;

pub struct RecordingEngineWebhookUseCase {
    repository: Arc<dyn RecordingEngineWebhookRepository + Send + Sync>,
    job_repository: Arc<dyn JobRepository + Send + Sync>,
    cover_storage: Arc<dyn CoverStorageClient + Send + Sync>,
    allowed_recording_base: PathBuf,
}

impl RecordingEngineWebhookUseCase {
    pub fn new(
        repository: Arc<dyn RecordingEngineWebhookRepository + Send + Sync>,
        job_repository: Arc<dyn JobRepository + Send + Sync>,
        cover_storage: Arc<dyn CoverStorageClient + Send + Sync>,
        allowed_recording_base: PathBuf,
    ) -> Self {
        Self {
            repository,
            job_repository,
            cover_storage,
            allowed_recording_base,
        }
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

        let poster_storage_path = self
            .generate_and_upload_cover_from_video(recording_id, &storage_path)
            .await?;

        // Update recording status to WaitingUpload and store local path
        let path_str = storage_path.to_string_lossy().into_owned();
        let changeset = RecordingTransmuxUpdateEntity {
            storage_path: Some(path_str.clone()),
            duration_sec,
            status: RecordingStatus::WaitingUpload.to_string(),
            updated_at: Utc::now(),
            poster_storage_path: Some(Some(poster_storage_path)),
        };

        let updated_recording_id = self
            .repository
            .update_live_transmux_finish(recording_id, changeset)
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

    async fn generate_and_upload_cover_from_video(
        &self,
        recording_id: Uuid,
        video_output_path: &Path,
    ) -> Result<String> {
        if !video_output_path.exists() {
            bail!(
                "transmux output path does not exist: {}",
                video_output_path.display()
            );
        }

        let thumbnail_path = self
            .generate_thumbnail_image(video_output_path, recording_id)
            .await?;

        let bytes = fs::read(&thumbnail_path).with_context(|| {
            format!(
                "failed to read generated thumbnail: {}",
                thumbnail_path.display()
            )
        })?;

        let stored_path = self
            .cover_storage
            .upload_cover(recording_id, bytes, "image/jpeg")
            .await?;

        if let Err(err) = fs::remove_file(&thumbnail_path) {
            warn!(
                path = %thumbnail_path.display(),
                "failed to remove temporary thumbnail file: {err:?}"
            );
        }

        Ok(stored_path)
    }

    async fn generate_thumbnail_image(
        &self,
        video_output_path: &Path,
        recording_id: Uuid,
    ) -> Result<PathBuf> {
        let temp_thumbnail_path = std::env::temp_dir().join(format!(
            "recording-thumb-{}-{}.jpg",
            recording_id,
            Uuid::new_v4()
        ));

        let output = Command::new("ffmpeg")
            .arg("-y")
            .arg("-ss")
            .arg("00:00:02")
            .arg("-i")
            .arg(video_output_path)
            .arg("-vframes")
            .arg("1")
            .arg("-vf")
            .arg("scale=640:-1")
            .arg("-q:v")
            .arg("3")
            .arg(&temp_thumbnail_path)
            .output()
            .await
            .context("failed to run ffmpeg for thumbnail generation")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(
                recording_id = %recording_id,
                video = %video_output_path.display(),
                status = %output.status,
                stderr = %stderr,
                "ffmpeg thumbnail generation failed"
            );
            bail!("ffmpeg thumbnail generation failed");
        }

        info!(
            recording_id = %recording_id,
            video = %video_output_path.display(),
            thumbnail = %temp_thumbnail_path.display(),
            "generated thumbnail from recording"
        );

        Ok(temp_thumbnail_path)
    }
}
