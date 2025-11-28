use anyhow::Result;
use domain::{
    entities::live_accounts::LiveAccountEntity,
    repositories::recording_engine_webhook::RecordingJobRepository,
    value_objects::enums::live_account_statuses::LiveAccountStatus,
};
use std::sync::Arc;
use uuid::Uuid;

pub struct RecordingEngineWebhookUseCase {
    repository: Arc<dyn RecordingJobRepository + Send + Sync>,
}

impl RecordingEngineWebhookUseCase {
    pub fn new(repository: Arc<dyn RecordingJobRepository + Send + Sync>) -> Self {
        Self { repository }
    }

    pub async fn get_unsynced_live_accounts(&self) -> Result<Vec<LiveAccountEntity>> {
        self.repository.find_unsynced_live_accounts().await
    }

    pub async fn update_live_account_status(
        &self,
        id: Uuid,
        status: LiveAccountStatus,
    ) -> Result<Uuid> {
        self.repository.update_live_account_status(id, status).await
    }
}
