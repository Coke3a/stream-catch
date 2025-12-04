use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::domain::{entities::live_accounts::LiveAccountEntity, value_objects::enums::live_account_statuses::LiveAccountStatus};

#[async_trait]
#[automock]
pub trait LiveAccountRecordingEngineRepository {
    async fn find_unsynced_live_accounts(&self) -> Result<Vec<LiveAccountEntity>>;
    async fn update_live_account_status(
        &self,
        live_account_id: Uuid,
        status: LiveAccountStatus,
    ) -> Result<Uuid>;
}   