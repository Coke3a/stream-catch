pub mod services;
pub mod webhook_server;

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
    let dotenvy_env = config::config_loader::load()?;
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
    let worker_loop = tokio::spawn(services::worker_loop::run_worker_loop(worker_usecase));

    let server_usecase = Arc::clone(&usecase);
    let server_port = dotenvy_env.worker_server.port;
    let webhook_server = tokio::spawn(async move {
        webhook_server::start_webhook_server(server_usecase, server_port).await
    });

    tokio::select! {
        result = worker_loop => result??,
        result = webhook_server => result??,
    };

    Ok(())
}
