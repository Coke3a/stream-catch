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

    let repository = Arc::new(RecordingJobPostgres::new(Arc::new(postgres_pool)));
    let supabase_storage = SupabaseStorageConfig {
        project_url: dotenvy_env.supabase.project_url.clone(),
        service_key: dotenvy_env.supabase.jwt_secret.clone(),
        poster_bucket: dotenvy_env.supabase.poster_bucket.clone(),
    };
    let usecase = Arc::new(RecordingEngineWebhookUseCase::new(
        repository,
        supabase_storage,
    ));

    info!("Worker started");

    let worker_usecase = Arc::clone(&usecase);
    let worker_loop = tokio::spawn(background_worker::worker_loop::run_worker_loop(
        worker_usecase,
    ));

    let server_config = Arc::clone(&dotenvy_env);
    let server_usecase = Arc::clone(&usecase);
    let webhook_server =
        tokio::spawn(
            async move { axum_http::http_serve::start(server_config, server_usecase).await },
        );

    tokio::select! {
        result = worker_loop => result??,
        result = webhook_server => result??,
    };

    Ok(())
}
