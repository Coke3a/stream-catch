use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::{domain, infra::db::postgres::{postgres_connection::PgPoolSquad, schema::jobs}};
use domain::{
    entities::jobs::{InsertJobEntity, JobEntity},
    repositories::job::JobRepository,
    value_objects::recording_upload::RecordingUploadPayload,
};

pub struct JobPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl JobPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl JobRepository for JobPostgres {
    async fn enqueue_recording_upload_job(
        &self,
        recording_id: Uuid,
        local_path: String,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let payload = RecordingUploadPayload {
            recording_id,
            local_path,
        };
        let payload_json = serde_json::to_value(payload)?;

        let insert_entity = InsertJobEntity {
            type_: "RecordingUpload".to_string(),
            payload: payload_json,
            run_at: Utc::now(),
            attempts: 0,
            locked_at: None,
            locked_by: None,
            status: "queued".to_string(),
            error: None,
            created_at: Utc::now(),
        };

        let result = diesel::insert_into(jobs::table)
            .values(&insert_entity)
            .returning(jobs::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn lock_next_recording_upload_job(&self) -> Result<Option<JobEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let worker_id = Uuid::new_v4().to_string();
        let current_time = Utc::now();

        // Using a transaction to lock the job
        let job = conn.transaction::<Option<JobEntity>, diesel::result::Error, _>(|conn| {
            // Find a candidate job
            // We use raw SQL for FOR UPDATE SKIP LOCKED because Diesel support might vary or be verbose
            // But let's try to use Diesel DSL if possible.
            // Assuming Postgres, we can use .for_update().skip_locked()

            let candidate: Option<JobEntity> = jobs::table
                .select(JobEntity::as_select())
                .filter(jobs::type_.eq("RecordingUpload"))
                .filter(jobs::status.eq("queued"))
                .filter(jobs::run_at.le(current_time))
                .order(jobs::run_at.asc())
                .for_update()
                .skip_locked()
                .first::<JobEntity>(conn)
                .optional()?;

            if let Some(job) = candidate {
                let updated_job = diesel::update(jobs::table.find(job.id))
                    .set((
                        jobs::status.eq("running"),
                        jobs::locked_at.eq(Some(current_time)),
                        jobs::locked_by.eq(Some(worker_id)),
                    ))
                    .returning(JobEntity::as_select())
                    .get_result::<JobEntity>(conn)?;
                Ok(Some(updated_job))
            } else {
                Ok(None)
            }
        })?;

        Ok(job)
    }

    async fn mark_job_done(&self, job_id: Uuid) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        diesel::update(jobs::table.find(job_id))
            .set((
                jobs::status.eq("done"),
                jobs::locked_at.eq::<Option<chrono::DateTime<Utc>>>(None),
                jobs::locked_by.eq::<Option<String>>(None),
            ))
            .returning(JobEntity::as_select())
            .execute(&mut conn)?;

        Ok(())
    }

    async fn mark_job_failed(&self, job_id: Uuid, err: &str, max_attempts: i32) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let current_time = Utc::now();

        // We need to fetch current attempts to decide if we retry or kill it
        // Or we can do it in a single query if we trust the logic.
        // Let's fetch first to be safe and simple.
        let job = jobs::table
            .find(job_id)
            .select(JobEntity::as_select())
            .first::<JobEntity>(&mut conn)?;

        let new_attempts = job.attempts + 1;
        let (new_status, next_run_at) = if new_attempts < max_attempts {
            // Exponential backoff: 5s, 25s, 125s...
            let backoff_sec = 5 * 5_i64.pow((new_attempts - 1) as u32);
            (
                "queued",
                current_time + chrono::Duration::seconds(backoff_sec),
            )
        } else {
            ("dead", current_time)
        };

        diesel::update(jobs::table.find(job_id))
            .set((
                jobs::status.eq(new_status),
                jobs::attempts.eq(new_attempts),
                jobs::error.eq(Some(err)),
                jobs::run_at.eq(next_run_at),
                jobs::locked_at.eq::<Option<chrono::DateTime<Utc>>>(None),
                jobs::locked_by.eq::<Option<String>>(None),
            ))
            .execute(&mut conn)?;

        Ok(())
    }
}
