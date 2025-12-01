use anyhow::Result;
use application::usercases::recording_engine_webhook::RecordingEngineWebhookUseCase;
use domain::value_objects::enums::live_account_statuses::LiveAccountStatus;
use std::{sync::Arc, time::Duration};
use tracing::{error, info};
use crate::services::web_driver_service;

pub async fn run_worker_loop(usecase: Arc<RecordingEngineWebhookUseCase>) -> Result<()> {
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

                if urls.ends_with('\n') {
                    urls.pop();
                }

                info!("Found unsynced accounts. URLs:\n{}", urls);

                // todo: 
                // match web_driver_service::add_account_recording_engine(urls, accounts).await {
                //     Ok(_) => info!("Successfully added accounts to Recording Engine"),
                //     Err(e) => error!("Failed to add accounts to Recording Engine: {}", e),
                // }

                for account in accounts {
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
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
