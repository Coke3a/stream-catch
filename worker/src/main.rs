use anyhow::Result;
use serde_json::json;
use thirtyfour::extensions::cdp::ChromeDevTools;
use thirtyfour::prelude::ElementWaitable;
use thirtyfour::{By, DesiredCapabilities, WebDriver};
use tracing::debug;
use tracing::error;
use tracing_subscriber::fmt;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    fmt().with_max_level(tracing::Level::DEBUG).init();

    if let Err(error) = worker::run().await {
        error!("Worker exited with error: {}", error);
        std::process::exit(1);
    }

    Ok(())
}
