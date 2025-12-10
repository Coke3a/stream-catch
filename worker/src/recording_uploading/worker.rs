use anyhow::{Context, Result};
use crates::domain::{
    entities::jobs::JobEntity,
    repositories::{
        job::JobRepository, recording_upload::RecordingUploadRepository, storage::StorageClient,
    },
    value_objects::recording_upload::RecordingUploadPayload,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tracing::{error, info, warn};

pub async fn run(
    job_repo: Arc<dyn JobRepository + Send + Sync>,
    recording_repo: Arc<dyn RecordingUploadRepository + Send + Sync>,
    storage: Arc<dyn StorageClient + Send + Sync>,
) -> Result<()> {
    info!("recording_upload: starting worker loop");
    loop {
        match job_repo.lock_next_recording_upload_job().await {
            Ok(Some(job)) => {
                info!(job_id = %job.id, "recording_upload: processing job");
                if let Err(e) =
                    process_recording_upload_job(&job_repo, &recording_repo, &storage, &job).await
                {
                    error!(
                        job_id = %job.id,
                        error = %e,
                        "recording_upload: failed to process job"
                    );
                    if let Err(mark_err) = job_repo
                        .mark_job_failed(job.id, &e.to_string(), 5) // MAX_ATTEMPTS = 5
                        .await
                    {
                        error!(
                            job_id = %job.id,
                            error = %mark_err,
                            "recording_upload: failed to mark job as failed"
                        );
                    }
                } else {
                    info!(job_id = %job.id, "recording_upload: job processed successfully");
                }
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                error!(
                    error = %e,
                    "recording_upload: error locking next job"
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn process_recording_upload_job(
    job_repo: &Arc<dyn JobRepository + Send + Sync>,
    recording_repo: &Arc<dyn RecordingUploadRepository + Send + Sync>,
    storage: &Arc<dyn StorageClient + Send + Sync>,
    job: &JobEntity,
) -> Result<()> {
    let payload: RecordingUploadPayload = serde_json::from_value(job.payload.clone())?;

    let local_path = canonicalize_path(&payload.local_path).map_err(|err| {
        error!(
            job_id = %job.id,
            path = %payload.local_path,
            error = %err,
            "recording_upload: failed to canonicalize path"
        );
        err
    })?;
    let local_path_str = local_path.to_string_lossy().into_owned();

    let recording = recording_repo
        .find_recording_by_id(payload.recording_id)
        .await
        .map_err(|err| {
            error!(
                job_id = %job.id,
                recording_id = %payload.recording_id,
                db_error = %err,
                "recording_upload: failed to fetch recording"
            );
            err
        })?
        .ok_or_else(|| {
            warn!(
                job_id = %job.id,
                recording_id = %payload.recording_id,
                "recording_upload: recording not found"
            );
            anyhow::anyhow!("recording not found")
        })?;

    info!(
        job_id = %job.id,
        recording_id = %recording.id,
        path = %local_path_str,
        "recording_upload: starting upload to storage"
    );
    let upload_result = storage
        .upload_recording(&local_path_str, &recording)
        .await
        .map_err(|err| {
            error!(
                job_id = %job.id,
                recording_id = %recording.id,
                path = %local_path_str,
                error = %err,
                "recording_upload: storage upload failed"
            );
            err
        })?;

    info!(
        job_id = %job.id,
        recording_id = %recording.id,
        remote_prefix = %upload_result.remote_prefix,
        size_bytes = upload_result.size_bytes,
        duration_sec = upload_result.duration_sec,
        "recording_upload: storage upload completed"
    );

    recording_repo
        .mark_recording_ready(
            recording.id,
            upload_result.remote_prefix,
            upload_result.size_bytes,
            upload_result.duration_sec,
        )
        .await
        .map_err(|err| {
            error!(
                job_id = %job.id,
                recording_id = %recording.id,
                db_error = %err,
                "recording_upload: failed to mark recording ready"
            );
            err
        })?;

    job_repo.mark_job_done(job.id).await.map_err(|err| {
        error!(
            job_id = %job.id,
            error = %err,
            "recording_upload: failed to mark job done"
        );
        err
    })?;

    Ok(())
}

fn canonicalize_path(path: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(path);
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to canonicalize recording path: {path}"))?;

    Ok(canonical)
}
