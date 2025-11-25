use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    entities::app_users::{AppUserEntity, InsertAppUserEntity, UpdateAppUserEntity},
    value_objects::enums::user_statuses::UserStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppUserModel {
    pub id: Uuid,
    pub display_name: Option<String>,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
}

impl From<AppUserEntity> for AppUserModel {
    fn from(entity: AppUserEntity) -> Self {
        Self {
            id: entity.id,
            display_name: entity.display_name,
            status: match entity.status.as_str() {
                "blocked" => UserStatus::Blocked,
                "inactive" => UserStatus::Inactive,
                _ => UserStatus::Active,
            },
            created_at: entity.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertAppUserModel {
    pub id: Uuid,
    pub display_name: Option<String>,
}

impl InsertAppUserModel {
    pub fn to_entity(&self) -> InsertAppUserEntity {
        InsertAppUserEntity {
            id: self.id,
            display_name: self.display_name.clone(),
            status: UserStatus::Active.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAppUserModel {
    pub display_name: Option<String>,
    pub status: Option<UserStatus>,
}

impl UpdateAppUserModel {
    pub fn to_entity(&self) -> UpdateAppUserEntity {
        UpdateAppUserEntity {
            display_name: self.display_name.clone(),
            status: self.status.as_ref().map(|status| status.to_string()),
        }
    }
}
