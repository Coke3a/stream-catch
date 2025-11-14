use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::domain::{entities::{password_reset_tokens::{CreatePasswordResetTokenEntity, UpdateToUsedPasswordResetTokenEntity}, users::RegisterUserEntity}, value_objects::user_statuses::UserStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserModel {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub telegram_id: Option<i64>,
    pub status: UserStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserModel {
    pub email: String,
    pub username: String,
    pub password_hash: String,
}

impl RegisterUserModel {
    pub fn to_entity(&self) -> RegisterUserEntity {
        RegisterUserEntity {
            email: self.email.clone(),
            username: self.username.clone(),
            password_hash: self.password_hash.clone(),
            status: UserStatus::Active.to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePasswordResetTokenModel {
    user_id: i64,
    token_hash: String,
    expires_at: NaiveDateTime,
    used_at: Option<NaiveDateTime>,
}

impl CreatePasswordResetTokenModel {
    pub fn to_entity(&self) -> CreatePasswordResetTokenEntity {
        CreatePasswordResetTokenEntity {
            user_id: self.user_id,
            token_hash: self.token_hash.clone(),
            expires_at: self.expires_at,
            used_at: self.used_at,
            created_at: Utc::now().naive_utc(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateToUsedPasswordResetTokenModel {
    used_at: Option<NaiveDateTime>,
}

impl UpdateToUsedPasswordResetTokenModel {
    pub fn to_entity(&self) -> UpdateToUsedPasswordResetTokenEntity {
        UpdateToUsedPasswordResetTokenEntity {
            used_at: self.used_at,
        }
    }
}
