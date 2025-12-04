use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::{
    domain::entities::{
        live_accounts::LiveAccountEntity,
        recordings::{
            InsertRecordingEntity, RecordingEntity, RecordingTransmuxUpdateEntity,
        },
    },
    domain::value_objects::enums::{
        recording_statuses::RecordingStatus,
    },
};

#[async_trait]
#[automock]
pub trait RecordingEngineWebhookRepository {
    async fn find_recording_by_live_account_and_status(
        &self,
        platform: String,
        account_id: String,
        status: RecordingStatus,
    ) -> Result<Option<RecordingEntity>>;
    async fn find_live_account_by_platform_and_account_id(
        &self,
        platform: String,
        account_id: String,
    ) -> Result<Option<LiveAccountEntity>>;
    async fn insert(&self, insert_recording_entity: InsertRecordingEntity) -> Result<Uuid>;
    async fn update_live_transmux_finish(
        &self,
        recording_id: Uuid,
        changeset: RecordingTransmuxUpdateEntity,
    ) -> Result<Uuid>;
    async fn update_file_uploading(&self, recording_id: Uuid) -> Result<Uuid>;
}
