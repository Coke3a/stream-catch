use crate::services::web_driver_service;
use anyhow::Result;
use application::usercases::recording_engine_webhook::RecordingEngineWebhookUseCase;
use domain::{
    entities::live_accounts::LiveAccountEntity,
    value_objects::enums::live_account_statuses::LiveAccountStatus,
};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

pub async fn run_worker_loop(usecase: Arc<RecordingEngineWebhookUseCase>) -> Result<()> {
    loop {
        if let Err(e) = process_unsynced_live_accounts(&usecase).await {
            error!("Error while processing unsynced live accounts: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn process_unsynced_live_accounts(usecase: &RecordingEngineWebhookUseCase) -> Result<()> {
    info!("Checking for unsynced live accounts...");
    let accounts = match usecase.get_unsynced_live_accounts().await {
        Ok(accounts) => accounts,
        Err(e) => {
            error!("Error fetching unsynced accounts: {}", e);
            return Err(e);
        }
    };

    if accounts.is_empty() {
        info!("No unsynced accounts found.");
        return Ok(());
    }

    log_unsynced_account_urls(&accounts);
    let (added_accounts, failed_accounts) =
        web_driver_service::add_account_recording_engine(build_urls_string(&accounts), accounts)
            .await
            .map_err(|e| {
                error!("Failed to add accounts to Recording Engine: {}", e);
                e
            })?;

    update_synced_accounts(usecase, &added_accounts).await;
    log_failed_accounts(failed_accounts.as_deref());

    Ok(())
}

fn build_urls_string(accounts: &[LiveAccountEntity]) -> String {
    accounts
        .iter()
        .map(|a| a.canonical_url.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn log_unsynced_account_urls(accounts: &[LiveAccountEntity]) {
    let urls = build_urls_string(accounts);
    info!("Found unsynced accounts. URLs:\n{}", urls);
}

async fn update_synced_accounts(
    usecase: &RecordingEngineWebhookUseCase,
    added_accounts: &[LiveAccountEntity],
) {
    for account in added_accounts {
        info!(
            "Updating account {} ({}) to Synced",
            account.id, account.canonical_url
        );

        if let Err(e) = usecase
            .update_live_account_status(account.id, LiveAccountStatus::Synced)
            .await
        {
            error!("Failed to update account {}: {}", account.id, e);
        } else {
            info!("Successfully updated account {} to Synced", account.id);
        }
    }
}

fn log_failed_accounts(failed_accounts: Option<&[LiveAccountEntity]>) {
    if let Some(accounts) = failed_accounts {
        for account in accounts {
            error!(
                "Failed to add account {} to Recording Engine. account_id: {}, URL: {}",
                account.id, account.account_id, account.canonical_url
            );
        }
    }
}
