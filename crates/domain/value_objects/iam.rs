use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::entities::app_users::{AppUserEntity, UpdateAppUserEntity},
    domain::value_objects::enums::user_statuses::UserStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppUserModel {
    pub id: Uuid,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AppUserEntity> for AppUserModel {
    fn from(entity: AppUserEntity) -> Self {
        Self {
            id: entity.id,
            status: match entity.status.as_str() {
                "blocked" => UserStatus::Blocked,
                "inactive" => UserStatus::Inactive,
                _ => UserStatus::Active,
            },
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAppUserModel {
    pub status: Option<UserStatus>,
}

impl UpdateAppUserModel {
    pub fn to_entity(&self) -> UpdateAppUserEntity {
        UpdateAppUserEntity {
            status: self.status.as_ref().map(|status| status.to_string()),
            updated_at: Some(Utc::now()),
        }
    }
}
