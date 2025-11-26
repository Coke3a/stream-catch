use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::app_users;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = app_users)]
pub struct AppUserEntity {
    pub id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = app_users)]
pub struct InsertAppUserEntity {
    pub id: Uuid,
    pub status: String,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = app_users)]
pub struct UpdateAppUserEntity {
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}
