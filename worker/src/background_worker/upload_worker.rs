use anyhow::{Context, Result, bail};
use std::{env, path::PathBuf, sync::Arc, time::Duration};
use tracing::{error, info};

use application::interfaces::storage::StorageClient;
use domain::{
    entities::jobs::JobEntity,
    repositories::{job::JobRepository, recording_engine_webhook::RecordingJobRepository},
    value_objects::recording_upload::RecordingUploadPayload,
};

pub async fn run_recording_upload_worker_loop(
    job_repo: Arc<dyn JobRepository + Send + Sync>,
    recording_repo: Arc<dyn RecordingJobRepository + Send + Sync>,
    storage: Arc<dyn StorageClient + Send + Sync>,
) -> Result<()> {
    info!("Starting RecordingUpload worker loop");
    loop {
        match job_repo.lock_next_recording_upload_job().await {
            Ok(Some(job)) => {
                info!("Processing RecordingUpload job: {}", job.id);
                if let Err(e) =
                    process_recording_upload_job(&job_repo, &recording_repo, &storage, &job).await
                {
                    error!("Failed to process job {}: {}", job.id, e);
                    if let Err(mark_err) = job_repo
                        .mark_job_failed(job.id, &e.to_string(), 5) // MAX_ATTEMPTS = 5
                        .await
                    {
                        error!("Failed to mark job {} as failed: {}", job.id, mark_err);
                    }
                } else {
                    info!("Successfully processed job: {}", job.id);
                }
            }
            Ok(None) => {
                // No jobs, sleep
                info!("No jobs found.");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                error!("Error locking next job: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn process_recording_upload_job(
    job_repo: &Arc<dyn JobRepository + Send + Sync>,
    recording_repo: &Arc<dyn RecordingJobRepository + Send + Sync>,
    storage: &Arc<dyn StorageClient + Send + Sync>,
    job: &JobEntity,
) -> Result<()> {
    let payload: RecordingUploadPayload = serde_json::from_value(job.payload.clone())?;

    let local_path = validate_local_path(&payload.local_path)?;
    let local_path_str = local_path.to_string_lossy().into_owned();

    let recording = recording_repo
        .find_recording_by_id(payload.recording_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("recording not found"))?;

    let upload_result = storage
        .upload_recording(&local_path_str, &recording)
        .await?;

    recording_repo
        .mark_recording_ready(
            recording.id,
            upload_result.remote_prefix,
            upload_result.size_bytes,
            upload_result.duration_sec,
        )
        .await?;

    job_repo.mark_job_done(job.id).await?;

    Ok(())
}

fn validate_local_path(path: &str) -> Result<PathBuf> {
    let base = env::var("RECORDING_LOCAL_BASE")
        .unwrap_or_else(|_| "/var/recordings".to_string());
    let base = PathBuf::from(base)
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
