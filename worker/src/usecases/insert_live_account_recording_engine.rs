use anyhow::Result;
use crates::domain;
use domain::{entities::live_accounts::LiveAccountEntity, repositories::live_account_recording_engine::LiveAccountRecordingEngineRepository, value_objects::enums::live_account_statuses::LiveAccountStatus};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

pub struct InsertLiveAccountUseCase {
    repository: Arc<dyn LiveAccountRecordingEngineRepository + Send + Sync>,
}

impl InsertLiveAccountUseCase {
    pub fn new(
        repository: Arc<dyn LiveAccountRecordingEngineRepository + Send + Sync>,
    ) -> Self {
        Self {
            repository,
        }
    }

    pub async fn get_unsynced_live_accounts(&self) -> Result<Vec<LiveAccountEntity>> {
        let accounts = self.repository.find_unsynced_live_accounts().await?;
        info!(count = accounts.len(), "found unsynced live accounts");
        Ok(accounts)
    }

    pub async fn update_live_account_status(
        &self,
        id: Uuid,
        status: LiveAccountStatus,
    ) -> Result<Uuid> {
        info!(live_account_id = %id, %status, "updating live account status");
        self.repository.update_live_account_status(id, status).await
    }
}