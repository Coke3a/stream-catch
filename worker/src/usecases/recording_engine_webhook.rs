use anyhow::{Context, Result, bail};
use chrono::Utc;
use crates::domain;
use domain::{
    entities::recordings::RecordingTransmuxUpdateEntity,
    repositories::recording_engine_webhook::RecordingEngineWebhookRepository,
    value_objects::{
        enums::{platforms::Platform, recording_statuses::RecordingStatus},
        recording_engine_webhook::{
            RecordingEngineErrorWebhook, RecordingEngineLiveStartWebhook,
            RecordingEngineTransmuxFinishWebhook,
        },
        recordings::InsertRecordingModel,
    },
};
use mp4::Mp4Reader;
use std::{
    fs::{self, File},
    io::BufReader,
    path::{Component, Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::config_model::RecordingEnginePaths;
use domain::repositories::job::JobRepository;
use domain::repositories::storage::CoverStorageClient;

pub struct RecordingEngineWebhookUseCase {
    repository: Arc<dyn RecordingEngineWebhookRepository + Send + Sync>,
    job_repository: Arc<dyn JobRepository + Send + Sync>,
    cover_storage: Arc<dyn CoverStorageClient + Send + Sync>,
    recording_engine_paths: RecordingEnginePaths,
}

impl RecordingEngineWebhookUseCase {
    pub fn new(
        repository: Arc<dyn RecordingEngineWebhookRepository + Send + Sync>,
        job_repository: Arc<dyn JobRepository + Send + Sync>,
        cover_storage: Arc<dyn CoverStorageClient + Send + Sync>,
        recording_engine_paths: RecordingEnginePaths,
    ) -> Self {
        Self {
            repository,
            job_repository,
            cover_storage,
            recording_engine_paths,
        }
    }

    pub async fn handle_live_start(
        &self,
        payload: RecordingEngineLiveStartWebhook,
    ) -> Result<Uuid> {
        info!(
            payload_id = %payload.id,
            payload = ?payload,
            "handling live_start webhook"
        );
        let data = payload.data;
        let platform = self.parse_platform(data.platform)?;
        let channel = data
            .channel
            .clone()
            .ok_or_else(|| anyhow::anyhow!("channel is required"))?;

        let live_account = self
            .repository
            .find_live_account_by_platform_and_account_id(platform.to_string(), channel.clone())
            .await
            .map_err(|err| {
                error!(
                    platform = %platform,
                    channel,
                    db_error = ?err,
                    "live_start: failed to find live account"
                );
                err
            })?
            .ok_or_else(|| {
                warn!(
                    platform = %platform,
                    channel,
                    "live_start: live account not found for platform/channel"
                );
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
        let recording_id = self.repository.insert(insert_entity).await.map_err(|err| {
            error!(
                platform = %platform,
                channel,
                db_error = ?err,
                "live_start: failed to insert recording"
            );
            err
        })?;
        info!(%recording_id, "live_start: recording inserted");
        Ok(recording_id)
    }

    pub async fn handle_transmux_finish(
        &self,
        payload: RecordingEngineTransmuxFinishWebhook,
    ) -> Result<Uuid> {
        info!(
            payload_id = %payload.id,
            payload = ?payload,
            "handling video_transmux_finish webhook"
        );
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
        let storage_path =
            Self::container_to_host_path(&storage_path_raw, &self.recording_engine_paths)?;

        let recording_id = if let Some(recording) = self
            .repository
            .find_recording_by_live_account_and_status(
                platform_string.clone(),
                channel.clone(),
                RecordingStatus::LiveRecording,
            )
            .await
            .map_err(|err| {
                error!(
                    platform = %platform_string,
                    channel,
                    db_error = ?err,
                    "transmux_finish: failed to find live recording by status"
                );
                err
            })? {
            recording.id
        } else {
            let live_account = self
                .repository
                .find_live_account_by_platform_and_account_id(
                    platform_string.clone(),
                    channel.clone(),
                )
                .await
                .map_err(|err| {
                    error!(
                        platform = %platform_string,
                        channel,
                        db_error = ?err,
                        "transmux_finish: failed to load live account"
                    );
                    err
                })?
                .ok_or_else(|| {
                    warn!(
                        platform = %platform_string,
                        channel,
                        "transmux_finish: live account not found for platform/channel"
                    );
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

            let new_id = self
                .repository
                .insert(insert_model.to_entity())
                .await
                .map_err(|err| {
                    error!(
                        platform = %platform_string,
                        channel,
                        db_error = ?err,
                        "transmux_finish: failed to insert placeholder recording"
                    );
                    err
                })?;
            info!(%new_id, "transmux_finish: inserted placeholder recording");
            new_id
        };

        let duration_sec = if Self::is_mp4_path(&storage_path) {
            match Self::read_mp4_duration_seconds(storage_path.clone()).await {
                Ok(duration) => Some(duration),
                Err(err) => {
                    error!(
                        path = %storage_path.display(),
                        error = ?err,
                        "failed to read duration for mp4 output"
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
            .await
            .map_err(|err| {
                error!(
                    %recording_id,
                    path = %storage_path.display(),
                    error = ?err,
                    "transmux_finish: failed to generate/upload cover"
                );
                err
            })?;

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
            .await
            .map_err(|err| {
                error!(
                    %recording_id,
                    db_error = ?err,
                    "transmux_finish: failed to update recording status"
                );
                err
            })?;

        // Enqueue upload job
        self.job_repository
            .enqueue_recording_upload_job(updated_recording_id, path_str)
            .await
            .map_err(|err| {
                error!(
                    %updated_recording_id,
                    job_error = ?err,
                    "transmux_finish: failed to enqueue upload job"
                );
                err
            })?;

        info!(%updated_recording_id, "transmux_finish: enqueued upload job and updated recording");

        Ok(updated_recording_id)
    }

    pub async fn handle_uploading_status(
        &self,
        platform: Option<String>,
        channel: Option<String>,
    ) -> Result<Uuid> {
        info!(
            platform = ?platform,
            channel = ?channel,
            "handling video_uploading webhook"
        );
        let parsed_platform = self.parse_platform(platform)?;
        let channel = channel.ok_or_else(|| anyhow::anyhow!("channel is required"))?;

        let recording = self
            .repository
            .find_recording_by_live_account_and_status(
                parsed_platform.to_string(),
                channel.clone(),
                RecordingStatus::WaitingUpload,
            )
            .await
            .map_err(|err| {
                error!(
                    platform = %parsed_platform,
                    channel,
                    db_error = ?err,
                    "uploading_status: failed to load recording"
                );
                err
            })?
            .ok_or_else(|| {
                warn!(
                    platform = %parsed_platform,
                    channel,
                    "uploading_status: recording not found in waiting_upload"
                );
                anyhow::anyhow!(
                    "Recording not found for platform {} and channel {} in status waiting_upload",
                    parsed_platform,
                    channel
                )
            })?;

        info!(recording_id = %recording.id, "marking recording as uploading");
        self.repository
            .update_file_uploading(recording.id)
            .await
            .map_err(|err| {
                error!(
                    recording_id = %recording.id,
                    db_error = ?err,
                    "uploading_status: failed to mark file uploading"
                );
                err
            })
    }

    pub async fn handle_error(&self, payload: RecordingEngineErrorWebhook) -> Result<Uuid> {
        let data = payload.data;
        let platform = data.platform.as_deref().unwrap_or("unknown");
        let channel = data.channel.as_deref().unwrap_or("unknown");
        let error_message = data.error.as_deref().unwrap_or("missing error");

        warn!(
            payload_id = %payload.id,
            payload_ts = %payload.ts,
            payload_type = %payload.type_,
            platform,
            channel,
            error = %error_message,
            "recording_engine_webhook: error received"
        );

        Ok(payload.id)
    }

    fn parse_platform(&self, platform: Option<String>) -> Result<Platform> {
        let platform_str = platform.ok_or_else(|| {
            warn!("webhook: platform is required but missing in payload");
            anyhow::anyhow!("platform is required")
        })?;
        Platform::from_str(&platform_str).map_err(|_| {
            warn!(
                platform = %platform_str,
                "webhook: unsupported platform received"
            );
            anyhow::anyhow!("Unsupported platform: {}", platform_str)
        })
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

    fn container_to_host_path(container_path: &str, paths: &RecordingEnginePaths) -> Result<PathBuf> {
        let prefix = Path::new(paths.container_prefix.as_str());
        let container_path = Path::new(container_path);
        let relative = container_path.strip_prefix(prefix).map_err(|_| {
            anyhow::anyhow!(
                "transmux output path is not under configured container prefix {}: {}",
                paths.container_prefix,
                container_path.display()
            )
        })?;

        if relative
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            bail!(
                "transmux output path contains invalid traversal segments: {}",
                container_path.display()
            );
        }

        Ok(Path::new("/rec").join(relative))
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

        info!(
            %recording_id,
            path = %video_output_path.display(),
            "cover: generating thumbnail and uploading"
        );

        let thumbnail_path = self
            .generate_thumbnail_image(video_output_path, recording_id)
            .await
            .map_err(|err| {
                error!(
                    %recording_id,
                    path = %video_output_path.display(),
                    error = ?err,
                    "cover: thumbnail generation failed"
                );
                err
            })?;

        let bytes = fs::read(&thumbnail_path).with_context(|| {
            format!(
                "failed to read generated thumbnail: {}",
                thumbnail_path.display()
            )
        })?;

        let stored_path = self
            .cover_storage
            .upload_cover(recording_id, bytes, "image/jpeg")
            .await
            .map_err(|err| {
                error!(
                    %recording_id,
                    path = %thumbnail_path.display(),
                    error = ?err,
                    "cover: failed to upload cover to storage"
                );
                err
            })?;

        if let Err(err) = fs::remove_file(&thumbnail_path) {
            warn!(
                path = %thumbnail_path.display(),
                "failed to remove temporary thumbnail file: {err:?}"
            );
        }

        info!(
            %recording_id,
            stored_path = %stored_path,
            "cover: thumbnail uploaded"
        );

        Ok(stored_path)
    }

    async fn generate_thumbnail_image(
        &self,
        video_output_path: &Path,
        recording_id: Uuid,
    ) -> Result<PathBuf> {
        info!(
            recording_id = %recording_id,
            video = %video_output_path.display(),
            "cover: starting thumbnail generation"
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_paths() -> RecordingEnginePaths {
        RecordingEnginePaths {
            container_prefix: "/app/rec".to_string(),
        }
    }

    #[test]
    fn container_to_host_path_maps_under_prefix() {
        let mapped = RecordingEngineWebhookUseCase::container_to_host_path(
            "/app/rec/tiktok/chan/2025-12-19/video.mp4",
            &test_paths(),
        )
        .expect("expected valid path mapping");

        assert_eq!(
            mapped,
            PathBuf::from("/rec/tiktok/chan/2025-12-19/video.mp4")
        );
    }

    #[test]
    fn container_to_host_path_rejects_outside_prefix() {
        let err = RecordingEngineWebhookUseCase::container_to_host_path(
            "/other/rec/tiktok/video.mp4",
            &test_paths(),
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("not under configured container prefix"));
    }

    #[test]
    fn container_to_host_path_rejects_traversal() {
        let err = RecordingEngineWebhookUseCase::container_to_host_path(
            "/app/rec/../secrets.mp4",
            &test_paths(),
        )
        .unwrap_err();

        assert!(err.to_string().contains("invalid traversal"));
    }
}
