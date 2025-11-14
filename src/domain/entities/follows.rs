use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::infrastructure::postgres::schema::follows;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(primary_key(user_id, live_account_id))]
#[diesel(table_name = follows)]
pub struct FollowEntity {
    pub user_id: i64,
    pub live_account_id: i64,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}


#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = follows)]
pub struct InsertFollowEntity {
    pub user_id: i64,
    pub live_account_id: i64,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, AsChangeset, Queryable)]
#[diesel(table_name = follows)]
pub struct EditFollowEntity {
    pub status: String,
    pub updated_at: NaiveDateTime,
}
