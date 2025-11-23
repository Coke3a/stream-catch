use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    entities::{follows::{EditFollowEntity}},
    value_objects::enums::{follow_statuses::FollowStatus, live_account_statuses::LiveAccountStatus, platforms::Platform, sort_order::SortOrder},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FollowModel {
    pub id: i64,
    pub user_id: Uuid,
    pub live_account_id: i64,
    pub status: FollowStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusFollowModel {
    pub status: FollowStatus,
}

impl UpdateStatusFollowModel {
    pub fn to_entity(&self) -> EditFollowEntity {
        EditFollowEntity {
            status: self.status.to_string(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAccountModel {
    pub id: i64,
    pub platform: Platform,
    pub account_id: String,
    pub canonical_url: String,
    pub status: LiveAccountStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindLiveAccountModel {
    pub platform: Platform,
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListFollowsFilter {
    pub live_account_id: Option<String>,
    pub platform: Option<Platform>,
    pub status: Option<FollowStatus>,
    pub limit: Option<i64>,
    pub sort_order: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsertFollowLiveAccountModel {
    pub url: String,
}
