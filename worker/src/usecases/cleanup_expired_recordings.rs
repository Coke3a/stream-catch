use anyhow::Result;
use chrono::{Duration, Utc};
use crates::domain::repositories::{
    recording_cleanup::RecordingCleanupRepository,
    storage::{CoverStorageClient, StorageClient},
};
use std::sync::Arc;
use tracing::warn;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CleanupExpiredRecordingsParams {
    pub older_than_days: i64,
    pub limit: Option<i64>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CleanupExpiredRecordingsResult {
    pub scanned: usize,
    pub deleted: usize,
    pub skipped_video_delete_failed: usize,
    pub cover_delete_failed: usize,
    pub updated_db: usize,
    pub candidate_ids: Vec<Uuid>,
    pub deleted_ids: Vec<Uuid>,
    pub skipped_ids: Vec<Uuid>,
    pub cover_failed_ids: Vec<Uuid>,
}

pub struct CleanupExpiredRecordingsUseCase {
    repository: Arc<dyn RecordingCleanupRepository + Send + Sync>,
    video_storage: Arc<dyn StorageClient + Send + Sync>,
    cover_storage: Arc<dyn CoverStorageClient + Send + Sync>,
}

impl CleanupExpiredRecordingsUseCase {
    pub fn new(
        repository: Arc<dyn RecordingCleanupRepository + Send + Sync>,
        video_storage: Arc<dyn StorageClient + Send + Sync>,
        cover_storage: Arc<dyn CoverStorageClient + Send + Sync>,
    ) -> Self {
        Self {
            repository,
            video_storage,
            cover_storage,
        }
    }

    pub async fn run(
        &self,
        params: CleanupExpiredRecordingsParams,
    ) -> Result<CleanupExpiredRecordingsResult> {
        let older_than_days = params.older_than_days.max(0);
        let older_than = Utc::now() - Duration::days(older_than_days);
        let limit = params.limit.filter(|l| *l > 0);

        let recordings = self
            .repository
            .list_expired_ready_recordings(older_than, limit)
            .await?;

        let mut result = CleanupExpiredRecordingsResult {
            scanned: recordings.len(),
            ..Default::default()
        };

        for recording in recordings {
            let Some(storage_path) = recording.storage_path.clone() else {
                continue;
            };

            if result.candidate_ids.len() < 20 {
                result.candidate_ids.push(recording.id);
            }

            if params.dry_run {
                continue;
            }

            // next run must be idempotent and continue to the DB update even if the object is
            // already missing.
            let video_deleted = match self.video_storage.delete_object(&storage_path).await {
                Ok(()) => true,
                Err(err) if looks_like_missing_object_error(&err) => {
                    warn!(
                        recording_id = %recording.id,
                        storage_path = %storage_path,
                        error = ?err,
                        "cleanup_recordings: video object already missing; continuing"
                    );
                    true
                }
                Err(err) => {
                    error!(
                        recording_id = %recording.id,
                        storage_path = %storage_path,
                        error = ?err,
                        "cleanup_recordings: failed to delete video object; skipping"
                    );
                    result.skipped_video_delete_failed += 1;
                    if result.skipped_ids.len() < 20 {
                        result.skipped_ids.push(recording.id);
                    }
                    continue;
                }
            };

            if !video_deleted {
                continue;
            }
            result.deleted += 1;
            if result.deleted_ids.len() < 20 {
                result.deleted_ids.push(recording.id);
            }

            if let Some(poster_storage_path) = recording.poster_storage_path.clone() {
                if let Err(err) = self.cover_storage.delete_object(&poster_storage_path).await {
                    error!(
                        recording_id = %recording.id,
                        poster_storage_path = %poster_storage_path,
                        error = ?err,
                        "cleanup_recordings: failed to delete cover object; will still update DB"
                    );
                    result.cover_delete_failed += 1;
                    if result.cover_failed_ids.len() < 20 {
                        result.cover_failed_ids.push(recording.id);
                    }
                }
            }

            match self
                .repository
                .mark_recording_expired_deleted(recording.id)
                .await
            {
                Ok(_) => result.updated_db += 1,
                Err(err) => {
                    error!(
                        recording_id = %recording.id,
                        error = ?err,
                        "cleanup_recordings: failed to update recording status in DB"
                    );
                }
            }
        }

        info!(
            scanned = result.scanned,
            deleted = result.deleted,
            skipped_video_delete_failed = result.skipped_video_delete_failed,
            cover_delete_failed = result.cover_delete_failed,
            updated_db = result.updated_db,
            dry_run = params.dry_run,
            "cleanup_recordings: completed"
        );

        Ok(result)
    }
}

fn looks_like_missing_object_error(err: &anyhow::Error) -> bool {
    // We only have `anyhow::Error` at this layer; keep the check conservative and avoid
    // hard-coding storage SDK types here.
    let message = err.to_string().to_ascii_lowercase();
    message.contains("status 404")
        || message.contains("nosuchkey")
        || message.contains("no such key")
        || message.contains("notfound")
}
