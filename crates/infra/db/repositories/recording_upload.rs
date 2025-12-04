use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::{OptionalExtension, RunQueryDsl, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::recordings},
};
use domain::{
    entities::recordings::RecordingEntity,
    repositories::recording_upload::RecordingUploadRepository,
    value_objects::enums::recording_statuses::RecordingStatus,
};

pub struct RecordingUploadPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl RecordingUploadPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl RecordingUploadRepository for RecordingUploadPostgres {
    async fn find_recording_by_id(&self, recording_id: Uuid) -> Result<Option<RecordingEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = recordings::table
            .find(recording_id)
            .select(RecordingEntity::as_select())
            .first::<RecordingEntity>(&mut conn)
            .optional()?;

        Ok(result)
    }

    async fn mark_recording_ready(
        &self,
        recording_id: Uuid,
        storage_path: String,
        size_bytes: i64,
        duration_sec: i32,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let result = update(recordings::table.filter(recordings::id.eq(recording_id)))
            .set((
                recordings::status.eq(RecordingStatus::Ready.to_string()),
                recordings::storage_path.eq(Some(storage_path)),
                recordings::size_bytes.eq(Some(size_bytes)),
                recordings::duration_sec.eq(Some(duration_sec)),
                recordings::updated_at.eq(now),
            ))
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }
}
