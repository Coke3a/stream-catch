use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{RunQueryDsl, prelude::*, update};
use std::sync::Arc;
use tokio::task;
use uuid::Uuid;

use crate::{
    domain::{
        entities::recordings::RecordingEntity,
        repositories::recording_cleanup::RecordingCleanupRepository,
        value_objects::enums::recording_statuses::RecordingStatus,
    },
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::recordings},
};

pub struct RecordingCleanupPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl RecordingCleanupPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl RecordingCleanupRepository for RecordingCleanupPostgres {
    async fn list_expired_ready_recordings(
        &self,
        older_than: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<RecordingEntity>> {
        // Issue #4: Diesel is synchronous; run DB work on the blocking threadpool to avoid
        // stalling Tokio under load.
        let db_pool = Arc::clone(&self.db_pool);

        Ok(task::spawn_blocking(move || -> Result<Vec<RecordingEntity>> {
            let mut conn = db_pool.get()?;

            let mut query = recordings::table
                .select(RecordingEntity::as_select())
                .filter(recordings::status.eq(RecordingStatus::Ready.to_string()))
                .filter(recordings::started_at.lt(older_than))
                .filter(recordings::storage_path.is_not_null())
                .order(recordings::started_at.asc())
                .into_boxed();

            if let Some(limit) = limit {
                query = query.limit(limit);
            }

            let result = query.load::<RecordingEntity>(&mut conn)?;
            Ok(result)
        })
        .await??)
    }

    async fn mark_recording_expired_deleted(&self, recording_id: Uuid) -> Result<Uuid> {
        // Issue #4: Diesel is synchronous; run DB work on the blocking threadpool to avoid
        // stalling Tokio under load.
        let db_pool = Arc::clone(&self.db_pool);
        let now = Utc::now();

        Ok(task::spawn_blocking(move || -> Result<Uuid> {
            let mut conn = db_pool.get()?;

            let updated_id = update(recordings::table.filter(recordings::id.eq(recording_id)))
                .set((
                    recordings::status.eq(RecordingStatus::ExpiredDeleted.to_string()),
                    recordings::updated_at.eq(now),
                ))
                .returning(recordings::id)
                .get_result::<Uuid>(&mut conn)?;

            Ok(updated_id)
        })
        .await??)
    }
}
