use anyhow::Result;
use backend::axum_http::http_serve;
use backend::config::config_loader;
use crates::infra::db::postgres::postgres_connection;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        error!("Backend exited with error: {}", error);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    crates::observability::init_observability("backend")?;

    let dotenvy_env = config_loader::load()?;
    info!("ENV has been loaded");

    let postgres_pool = postgres_connection::establish_connection(&dotenvy_env.database.url)?;
    info!("Postgres connection has been established");

    http_serve::start(Arc::new(dotenvy_env), Arc::new(postgres_pool)).await?;

    Ok(())
}
