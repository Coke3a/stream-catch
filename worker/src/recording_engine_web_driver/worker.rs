use anyhow::Result;
use crates::{domain, infra::web_driver::insert_live_account::add_account_recording_engine};
use domain::{
    entities::live_accounts::LiveAccountEntity,
    value_objects::enums::live_account_statuses::LiveAccountStatus,
};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

use crate::usecases::insert_live_account_recording_engine::InsertLiveAccountUseCase;

pub async fn run(usecase: Arc<InsertLiveAccountUseCase>) -> Result<()> {
    info!("Starting RecordingEngineWebDriver worker loop");
    loop {
        if let Err(e) = process_unsynced_live_accounts(&usecase).await {
            error!(
                error = %e,
                "recording_engine_web_driver: error while processing unsynced live accounts"
            );
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn process_unsynced_live_accounts(usecase: &InsertLiveAccountUseCase) -> Result<()> {
    let accounts = match usecase.get_unsynced_live_accounts().await {
        Ok(accounts) => accounts,
        Err(e) => {
            error!(
                error = %e,
                "recording_engine_web_driver: error fetching unsynced accounts"
            );
            return Err(e);
        }
    };

    if accounts.is_empty() {
        return Ok(());
    }

    log_unsynced_account_urls(&accounts);
    let (added_accounts, failed_accounts) =
        add_account_recording_engine(build_urls_string(&accounts), accounts)
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    "recording_engine_web_driver: failed to add accounts to Recording Engine"
                );
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
    info!(
        account_count = accounts.len(),
        urls = %urls,
        "Found unsynced accounts"
    );
}

async fn update_synced_accounts(
    usecase: &InsertLiveAccountUseCase,
    added_accounts: &[LiveAccountEntity],
) {
    for account in added_accounts {
        info!(
            live_account_id = %account.id,
            canonical_url = %account.canonical_url,
            "Updating account to Synced"
        );

        if let Err(e) = usecase
            .update_live_account_status(account.id, LiveAccountStatus::Synced)
            .await
        {
            error!(
                live_account_id = %account.id,
                error = %e,
                "Failed to update account to Synced"
            );
        } else {
            info!(
                live_account_id = %account.id,
                "Successfully updated account to Synced"
            );
        }
    }
}

fn log_failed_accounts(failed_accounts: Option<&[LiveAccountEntity]>) {
    if let Some(accounts) = failed_accounts {
        for account in accounts {
            error!(
                live_account_id = %account.id,
                account_id = %account.account_id,
                canonical_url = %account.canonical_url,
                "Failed to add account to Recording Engine"
            );
        }
    }
}
