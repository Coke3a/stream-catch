use anyhow::Result;
use crates::domain::repositories::{
    job::JobRepository,
    live_account_recording_engine::LiveAccountRecordingEngineRepository,
    recording_cleanup::RecordingCleanupRepository,
    recording_engine_webhook::RecordingEngineWebhookRepository,
    recording_upload::RecordingUploadRepository,
    storage::{CoverStorageClient, StorageClient},
};
use crates::infra::{
    db::{
        postgres::postgres_connection,
        repositories::{
            job::JobPostgres, live_account_recording_engine::LiveAccountRecordingEnginePostgres,
            recording_cleanup::RecordingCleanupPostgres,
            recording_engine_webhook::RecordingEngineWebhookPostgres,
            recording_upload::RecordingUploadPostgres,
        },
    },
    storages::{
        supabase_storage::{SupabaseStorageClient, SupabaseStorageConfig},
        wasabi::WasabiStorageClient,
    },
};
use std::sync::Arc;
use tracing::error;
use tracing::info;
use worker::{
    axum_http, config, recording_engine_web_driver, recording_uploading,
    usecases::{
        cleanup_expired_recordings::CleanupExpiredRecordingsUseCase,
        insert_live_account_recording_engine::InsertLiveAccountUseCase,
        recording_engine_webhook::RecordingEngineWebhookUseCase,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(error) = run().await {
        error!("Worker exited with error: {}", error);
        std::process::exit(1);
    }
    Ok(())
}

async fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    crates::observability::init_observability("worker")?;

    let dotenvy_env = Arc::new(config::config_loader::load()?);
    info!("ENV has been loaded");

    let postgres_pool = postgres_connection::establish_connection(&dotenvy_env.database.url)?;
    info!("Postgres connection has been established");

    let db_pool_arc = Arc::new(postgres_pool);

    // Create repository (shared DB pool)
    let live_account_repository: Arc<dyn LiveAccountRecordingEngineRepository + Send + Sync> =
        Arc::new(LiveAccountRecordingEnginePostgres::new(Arc::clone(
            &db_pool_arc,
        )));

    // Create usecase that depends on the repo
    let insert_live_account_usecase = Arc::new(InsertLiveAccountUseCase::new(Arc::clone(
        &live_account_repository,
    )));

    // Spawn background loop
    let recording_engine_web_driver_loop = tokio::spawn(recording_engine_web_driver::worker::run(
        insert_live_account_usecase,
    ));

    // init recording_engine_webhook
    let supa = &dotenvy_env.supabase;
    let cover_storage_client: Arc<dyn CoverStorageClient + Send + Sync> = Arc::new(
        SupabaseStorageClient::new(SupabaseStorageConfig {
            endpoint: supa.s3_endpoint.clone(),
            region: supa.s3_region.clone(),
            bucket: supa.poster_bucket.clone(),
            access_key: supa.s3_access_key.clone(),
            secret_key: supa.s3_secret_key.clone(),
            prefix: supa.poster_prefix.clone(),
        })
        .await?,
    );

    let recording_engine_webhook_repository: Arc<
        dyn RecordingEngineWebhookRepository + Send + Sync,
    > = Arc::new(RecordingEngineWebhookPostgres::new(Arc::clone(
        &db_pool_arc,
    )));

    let job_repository: Arc<dyn JobRepository + Send + Sync> =
        Arc::new(JobPostgres::new(Arc::clone(&db_pool_arc)));

    let recording_engine_webhook_usecase = Arc::new(RecordingEngineWebhookUseCase::new(
        recording_engine_webhook_repository,
        Arc::clone(&job_repository),
        Arc::clone(&cover_storage_client),
        dotenvy_env.recording_engine_paths.clone(),
    ));

    let server_config = Arc::clone(&dotenvy_env);
    let server_usecase = recording_engine_webhook_usecase;

    let cleanup_repo: Arc<dyn RecordingCleanupRepository + Send + Sync> =
        Arc::new(RecordingCleanupPostgres::new(Arc::clone(&db_pool_arc)));

    let recording_upload_repository: Arc<dyn RecordingUploadRepository + Send + Sync> =
        Arc::new(RecordingUploadPostgres::new(Arc::clone(&db_pool_arc)));

    let video_storage_client_config = dotenvy_env.video_storage.clone();
    let video_storage_client: Arc<dyn StorageClient + Send + Sync> =
        Arc::new(WasabiStorageClient::new(video_storage_client_config).await?);

    let cleanup_usecase = Arc::new(CleanupExpiredRecordingsUseCase::new(
        cleanup_repo,
        Arc::clone(&video_storage_client),
        Arc::clone(&cover_storage_client),
    ));

    let recording_engine_webhook = tokio::spawn(async move {
        axum_http::http_serve::start(server_config, server_usecase, cleanup_usecase).await
    });

    // Spawn background loop
    let recording_uploading_loop = tokio::spawn(recording_uploading::worker::run(
        job_repository,
        recording_upload_repository,
        Arc::clone(&video_storage_client),
        dotenvy_env.recording_upload.clone(),
    ));

    tokio::select! {
        result = recording_uploading_loop => result??,
        result = recording_engine_web_driver_loop => result??,
        result = recording_engine_webhook => result??,
    };
    Ok(())
}
