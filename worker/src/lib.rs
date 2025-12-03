pub mod axum_http;
pub mod background_worker;

use anyhow::Result;
use application::usercases::recording_engine_webhook::{
    RecordingEngineWebhookUseCase, SupabaseStorageConfig,
};
use backend::config;
use infra::postgres::{
    postgres_connection, repositories::recording_engine_webhook::RecordingJobPostgres,
};
use infra::storage::b2::{B2StorageClient, B2StorageConfig};
use std::sync::Arc;
use tracing::info;

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let dotenvy_env = Arc::new(config::config_loader::load()?);
    info!("ENV has been loaded");

    let postgres_pool = postgres_connection::establish_connection(&dotenvy_env.database.url)?;
    info!("Postgres connection has been established");

    let db_pool_arc = Arc::new(postgres_pool);
    let repository = Arc::new(RecordingJobPostgres::new(Arc::clone(&db_pool_arc)));
    let job_repository = Arc::new(infra::postgres::repositories::job::JobPostgres::new(
        Arc::clone(&db_pool_arc),
    ));

    let supabase_storage_config = SupabaseStorageConfig {
        project_url: dotenvy_env.supabase.project_url.clone(),
        service_key: dotenvy_env.supabase.jwt_secret.clone(),
        poster_bucket: dotenvy_env.supabase.poster_bucket.clone(),
    };

    // Config for StorageClient (Backblaze B2 via S3 API)
    let storage_client_config = B2StorageConfig::from_env()?;
    let storage_client = Arc::new(B2StorageClient::new(storage_client_config).await?);

    let usecase = Arc::new(RecordingEngineWebhookUseCase::new(
        repository.clone(),
        job_repository.clone(),
        supabase_storage_config,
    ));

    info!("Worker started");

    let worker_usecase = Arc::clone(&usecase);
    let worker_loop = tokio::spawn(background_worker::worker_loop::run_worker_loop(
        worker_usecase,
    ));

    let upload_worker_loop = tokio::spawn(
        background_worker::upload_worker::run_recording_upload_worker_loop(
            job_repository,
            repository,
            storage_client,
        ),
    );

    let server_config = Arc::clone(&dotenvy_env);
    let server_usecase = Arc::clone(&usecase);
    let webhook_server =
        tokio::spawn(
            async move { axum_http::http_serve::start(server_config, server_usecase).await },
        );

    tokio::select! {
        result = worker_loop => result??,
        result = upload_worker_loop => result??,
        result = webhook_server => result??,
    };

    Ok(())
}
