use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::value_objects::enums::{
        follow_statuses::FollowStatus, live_account_statuses::LiveAccountStatus,
        platforms::Platform, sort_order::SortOrder,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FollowModel {
    pub id: Uuid,
    pub user_id: Uuid,
    pub live_account_id: Uuid,
    pub status: FollowStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAccountModel {
    pub id: Uuid,
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
    pub live_account_id: Option<Uuid>,
    pub platform: Option<Platform>,
    pub status: Option<FollowStatus>,
    pub limit: Option<i64>,
    pub sort_order: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsertFollowLiveAccountModel {
    pub url: String,
}
