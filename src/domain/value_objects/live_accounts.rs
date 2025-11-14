use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{entities::{follows::{EditFollowEntity, InsertFollowEntity}, live_accounts::InsertLiveAccountEntity}, value_objects::{follow_statuses::FollowStatus, live_account_statuses::LiveAccountStatus, platforms::Platform}};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FollowModel {
    pub id: i64,
    pub user_id: i64,
    pub live_account_id: i64,
    pub status: FollowStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertFollowModel {
    pub user_id: i64,
    pub live_account_id: i64,
}

impl InsertFollowModel {
    pub fn to_entity(&self) -> InsertFollowEntity {
        InsertFollowEntity {
            user_id: self.user_id,
            live_account_id: self.live_account_id,
            status: FollowStatus::Active.to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }
    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusFollowModel {
    pub status: FollowStatus,
    pub updated_at: NaiveDateTime,
}

impl UpdateStatusFollowModel {
    pub fn to_entity(&self) -> EditFollowEntity {
        EditFollowEntity {
            status: self.status.to_string(),
            updated_at: self.updated_at,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsertLiveAccountModel {
    pub platform: Platform,
    pub account_id: String,
    pub canonical_url: String,
}

impl InsertLiveAccountModel {
    pub fn to_entity(&self) -> InsertLiveAccountEntity {
        InsertLiveAccountEntity {
            platform: self.platform.to_string(),
            account_id: self.account_id.clone(),
            canonical_url: self.canonical_url.clone(),
            status: LiveAccountStatus::Active.to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }
}
