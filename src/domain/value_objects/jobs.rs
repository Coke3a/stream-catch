use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{
    entities::jobs::{InsertJobEntity, JobEntity, UpdateJobEntity},
    value_objects::enums::{job_statuses::JobStatus, job_types::JobType},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobModel {
    pub id: i64,
    pub type_: JobType,
    pub payload: Value,
    pub run_at: DateTime<Utc>,
    pub attempts: i32,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub status: JobStatus,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsertJobModel {
    pub type_: JobType,
    pub payload: Value,
    pub run_at: DateTime<Utc>,
}

impl InsertJobModel {
    pub fn to_entity(&self) -> InsertJobEntity {
        InsertJobEntity {
            type_: self.type_.to_string(),
            payload: self.payload.clone(),
            run_at: self.run_at,
            attempts: 0,
            locked_at: None,
            locked_by: None,
            status: JobStatus::Queued.to_string(),
            error: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateJobModel {
    pub type_: Option<JobType>,
    pub payload: Option<Value>,
    pub run_at: Option<DateTime<Utc>>,
    pub attempts: Option<i32>,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub status: Option<JobStatus>,
    pub error: Option<String>,
}

impl UpdateJobModel {
    pub fn to_entity(&self) -> UpdateJobEntity {
        UpdateJobEntity {
            type_: self.type_.as_ref().map(|job_type| job_type.to_string()),
            payload: self.payload.clone(),
            run_at: self.run_at,
            attempts: self.attempts,
            locked_at: self.locked_at,
            locked_by: self.locked_by.clone(),
            status: self.status.as_ref().map(|status| status.to_string()),
            error: self.error.clone(),
        }
    }
}
