use anyhow::Result;
use axum::async_trait;
use mockall::automock;

use crate::domain::entities::follows::InsertFollowEntity;
use crate::domain::entities::live_accounts::{InsertLiveAccountEntity, LiveAccountEntity};
use crate::domain::value_objects::live_following::FindLiveAccountModel;

#[async_trait]
#[automock]
pub trait StreamFollowsRepository {
    async fn follow_and_create_live_account(&self, follow_entity: InsertFollowEntity, live_account_entry: InsertLiveAccountEntity) -> Result<i64>;
    async fn follow(&self, follow_entity: InsertFollowEntity) -> Result<i64>;
    async fn unfollow(&self, follow_id: i64) -> Result<()>;
    async fn find_live_account(&self, follow_entity: &FindLiveAccountModel) -> Result<LiveAccountEntity>;
}
