use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::infrastructure::postgres::schema::password_reset_tokens;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = password_reset_tokens)]
pub struct PasswordResetTokenEntity {
    pub id: i64,
    pub user_id: i64,
    pub token_hash: String,
    pub expires_at: NaiveDateTime,
    pub used_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[diesel(table_name = password_reset_tokens)]
pub struct CreatePasswordResetTokenEntity {
    pub user_id: i64,
    pub token_hash: String,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Queryable, AsChangeset)]
#[diesel(table_name = password_reset_tokens)]
pub struct UpdateToUsedPasswordResetTokenEntity {
    pub used_at: Option<NaiveDateTime>,
}
