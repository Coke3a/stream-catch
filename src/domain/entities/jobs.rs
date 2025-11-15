use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::infrastructure::postgres::schema::jobs;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = jobs)]
pub struct JobEntity {
    pub id: i64,
    pub type: String,
    pub payload: String,
    pub run_at: NaiveDateTime,
    pub attempts: i32,
    pub locked_at: NaiveDateTime,
    pub locked_by: String,
    pub status: String,
    pub error: String,
    pub created_at: NaiveDateTime,
}

