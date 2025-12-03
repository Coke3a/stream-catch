use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::follows;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(primary_key(user_id, live_account_id))]
#[diesel(table_name = follows)]
pub struct FollowEntity {
    pub user_id: Uuid,
    pub live_account_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = follows)]
pub struct InsertFollowEntity {
    pub user_id: Uuid,
    pub live_account_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
