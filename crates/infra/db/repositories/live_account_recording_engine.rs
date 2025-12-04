use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::{RunQueryDsl, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{domain, infra::db::postgres::{postgres_connection::PgPoolSquad, schema::live_accounts}};
use domain::{
    entities::live_accounts::LiveAccountEntity,
    repositories::live_account_recording_engine::LiveAccountRecordingEngineRepository,
    value_objects::enums::live_account_statuses::LiveAccountStatus,
};


pub struct LiveAccountRecordingEnginePostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl LiveAccountRecordingEnginePostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl LiveAccountRecordingEngineRepository for LiveAccountRecordingEnginePostgres {

    async fn find_unsynced_live_accounts(&self) -> Result<Vec<LiveAccountEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = live_accounts::table
            .filter(live_accounts::status.eq(LiveAccountStatus::Unsynced.to_string()))
            .select(LiveAccountEntity::as_select())
            .load::<LiveAccountEntity>(&mut conn)?;

        Ok(result)
    }

    async fn update_live_account_status(
        &self,
        live_account_id: Uuid,
        status: LiveAccountStatus,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let result = update(live_accounts::table.filter(live_accounts::id.eq(live_account_id)))
            .set((
                live_accounts::status.eq(status.to_string()),
                live_accounts::updated_at.eq(now),
            ))
            .returning(live_accounts::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }


}