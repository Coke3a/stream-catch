use anyhow::Result;
use tracing::error;
use tracing_subscriber::fmt;
use worker::services::web_driver_service;

#[tokio::main]
async fn main() -> Result<()> {
    fmt().with_max_level(tracing::Level::DEBUG).init();

    if let Err(error) = worker::run().await {
        error!("Worker exited with error: {}", error);
        std::process::exit(1);
    }

    // let insert_urls = "https://www.bigo.tv/ma_mint2545".to_string();
    // web_driver_service::add_account_recording_engine(insert_urls, vec![]).await;

    Ok(())
}
