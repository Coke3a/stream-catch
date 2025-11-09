use stream_catch::config::config_loader;
use tracing::{error, info};

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.init();
	
	let dotenvy_env = match config_loader::load() {
		Ok(env) => env,
		Err(e) => {
			error!("Failed to load env: {}", e);
			std::process::exit(1);
		}
	};
	
	info!("ENV has been loaded");
}
