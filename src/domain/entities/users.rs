use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::infrastructure::postgres::schema::users;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = users)]
pub struct UserEntity {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub telegram_id: i64,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = users)]
pub struct RegisterUserEntity {
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Queryable, AsChangeset)]
#[diesel(table_name = users)]
pub struct EditUserEntity {
    pub email: Option<String>,
    pub username: Option<String>,
    pub password_hash: Option<String>,
    pub telegram_id: Option<i64>,
    pub status: Option<String>,
    pub updated_at: NaiveDateTime,
}


