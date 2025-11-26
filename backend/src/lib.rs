pub mod axum_http;
pub mod config;

use std::sync::Arc;

use anyhow::Result;
use infra::postgres::postgres_connection;
use tracing::info;

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let dotenvy_env = config::config_loader::load()?;
    info!("ENV has been loaded");

    let postgres_pool = postgres_connection::establish_connection(&dotenvy_env.database.url)?;
    info!("Postgres connection has been established");

    axum_http::http_serve::start(Arc::new(dotenvy_env), Arc::new(postgres_pool)).await?;

    Ok(())
}
