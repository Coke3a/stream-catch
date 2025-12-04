use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::{OptionalExtension, RunQueryDsl, insert_into, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{
        postgres_connection::PgPoolSquad,
        schema::{live_accounts, recordings},
    },
};
use domain::{
    entities::{
        live_accounts::LiveAccountEntity,
        recordings::{InsertRecordingEntity, RecordingEntity, RecordingTransmuxUpdateEntity},
    },
    repositories::recording_engine_webhook::RecordingEngineWebhookRepository,
    value_objects::enums::recording_statuses::RecordingStatus,
};

pub struct RecordingEngineWebhookPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl RecordingEngineWebhookPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl RecordingEngineWebhookRepository for RecordingEngineWebhookPostgres {
    async fn find_recording_by_live_account_and_status(
        &self,
        platform: String,
        account_id: String,
        status: RecordingStatus,
    ) -> Result<Option<RecordingEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = recordings::table
            .inner_join(live_accounts::table.on(recordings::live_account_id.eq(live_accounts::id)))
            .select(RecordingEntity::as_select())
            .filter(live_accounts::platform.eq(platform))
            .filter(live_accounts::account_id.eq(account_id))
            .filter(recordings::status.eq(status.to_string()))
            .first::<RecordingEntity>(&mut conn)
            .optional()?;

        Ok(result)
    }

    async fn find_live_account_by_platform_and_account_id(
        &self,
        platform: String,
        account_id: String,
    ) -> Result<Option<LiveAccountEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = live_accounts::table
            .filter(live_accounts::platform.eq(platform))
            .filter(live_accounts::account_id.eq(account_id))
            .first::<LiveAccountEntity>(&mut conn)
            .optional()?;

        Ok(result)
    }

    async fn insert(&self, insert_recording_entity: InsertRecordingEntity) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = insert_into(recordings::table)
            .values(&insert_recording_entity)
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn update_live_transmux_finish(
        &self,
        recording_id: Uuid,
        changeset: RecordingTransmuxUpdateEntity,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let result = update(recordings::table.filter(recordings::id.eq(recording_id)))
            .set(changeset)
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn update_file_uploading(&self, recording_id: Uuid) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let result = update(recordings::table.filter(recordings::id.eq(recording_id)))
            .set((
                recordings::status.eq(RecordingStatus::Uploading.to_string()),
                recordings::updated_at.eq(now),
            ))
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }
}
