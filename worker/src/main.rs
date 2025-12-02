use anyhow::Result;
use tracing::error;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(error) = worker::run().await {
        error!("Worker exited with error: {}", error);
        std::process::exit(1);
    }
    Ok(())
}
