use anyhow::Result;
use application::usercases::recording_engine_webhook::RecordingEngineWebhookUseCase;
use backend::config;
use domain::value_objects::enums::live_account_statuses::LiveAccountStatus;
use infra::postgres::{
    postgres_connection, repositories::recording_engine_webhook::RecordingJobPostgres,
};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let dotenvy_env = config::config_loader::load()?;
    info!("ENV has been loaded");

    let postgres_pool = postgres_connection::establish_connection(&dotenvy_env.database.url)?;
    info!("Postgres connection has been established");

    let repository = Arc::new(RecordingJobPostgres::new(Arc::new(postgres_pool)));
    let usecase = RecordingEngineWebhookUseCase::new(repository);

    info!("Worker started");

    loop {
        info!("Checking for unsynced live accounts...");
        match usecase.get_unsynced_live_accounts().await {
            Ok(accounts) => {
                if accounts.is_empty() {
                    info!("No unsynced accounts found. Sleeping...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }

                let mut urls = String::new();
                for account in &accounts {
                    urls.push_str(&account.canonical_url);
                    urls.push('\n');
                }

                // Remove trailing newline if exists
                if urls.ends_with('\n') {
                    urls.pop();
                }

                info!("Found unsynced accounts. URLs:\n{}", urls);

                // TODO: Web scraping process (rust_web_scraper)
                // This will be implemented later.
                // For now, we simulate success for all accounts.

                // Simulate processing and updating status
                for account in accounts {
                    // Simulate success or failure logic here if needed
                    // For now, assume success and update to Synced
                    info!(
                        "Updating account {} ({}) to Synced",
                        account.id, account.canonical_url
                    );
                    match usecase
                        .update_live_account_status(account.id, LiveAccountStatus::Synced)
                        .await
                    {
                        Ok(_) => info!("Successfully updated account {} to Synced", account.id),
                        Err(e) => error!("Failed to update account {}: {}", account.id, e),
                    }
                }
            }
            Err(e) => {
                error!("Error fetching unsynced accounts: {}", e);
                // Sleep to avoid tight loop on DB error
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }

        // Sleep before next loop iteration
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
