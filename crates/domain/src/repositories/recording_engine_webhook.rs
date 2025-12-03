use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::{
    entities::{
        live_accounts::LiveAccountEntity,
        recordings::{InsertRecordingEntity, RecordingEntity},
    },
    value_objects::enums::{
        live_account_statuses::LiveAccountStatus, recording_statuses::RecordingStatus,
    },
};

#[async_trait]
#[automock]
pub trait RecordingJobRepository {
    async fn find_recording_by_live_account_and_status(
        &self,
        platform: String,
        account_id: String,
        status: RecordingStatus,
    ) -> Result<Option<RecordingEntity>>;
    async fn find_recording_by_id(&self, recording_id: Uuid) -> Result<Option<RecordingEntity>>;
    async fn find_live_account_by_platform_and_account_id(
        &self,
        platform: String,
        account_id: String,
    ) -> Result<Option<LiveAccountEntity>>;
    async fn insert(&self, insert_recording_entity: InsertRecordingEntity) -> Result<Uuid>;
    async fn update_live_transmux_finish(
        &self,
        recording_id: Uuid,
        storage_path: String,
        duration_sec: Option<i32>,
    ) -> Result<Uuid>;
    async fn update_file_uploading(&self, recording_id: Uuid) -> Result<Uuid>;
    async fn find_unsynced_live_accounts(&self) -> Result<Vec<LiveAccountEntity>>;
    async fn update_live_account_status(
        &self,
        live_account_id: Uuid,
        status: LiveAccountStatus,
    ) -> Result<Uuid>;
    async fn mark_recording_ready(
        &self,
        recording_id: Uuid,
        storage_path: String,
        size_bytes: i64,
        duration_sec: i32,
    ) -> Result<Uuid>;
}
