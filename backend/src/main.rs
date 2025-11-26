use tracing::error;

#[tokio::main]
async fn main() {
    if let Err(error) = backend::run().await {
        error!("Backend exited with error: {}", error);
        std::process::exit(1);
    }
}
