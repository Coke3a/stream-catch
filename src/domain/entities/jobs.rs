use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;

use crate::infrastructure::postgres::schema::jobs;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = jobs)]
pub struct JobEntity {
    pub id: i64,
    pub type_: String,
    pub payload: Value,
    pub run_at: DateTime<Utc>,
    pub attempts: i32,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = jobs)]
pub struct InsertJobEntity {
    pub type_: String,
    pub payload: Value,
    pub run_at: DateTime<Utc>,
    pub attempts: i32,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, AsChangeset, Queryable)]
#[diesel(table_name = jobs)]
pub struct UpdateJobEntity {
    pub type_: Option<String>,
    pub payload: Option<Value>,
    pub run_at: Option<DateTime<Utc>>,
    pub attempts: Option<i32>,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub status: Option<String>,
    pub error: Option<String>,
}
