use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::entities::follows::InsertFollowEntity;
use crate::entities::live_accounts::{InsertLiveAccountEntity, LiveAccountEntity};
use crate::value_objects::live_following::{FindLiveAccountModel, ListFollowsFilter};

#[async_trait]
#[automock]
pub trait LiveFollowingRepository {
    async fn follow_and_create_live_account(
        &self,
        follow_entity: InsertFollowEntity,
        live_account_entry: InsertLiveAccountEntity,
    ) -> Result<Uuid>;
    async fn follow(&self, follow_entity: InsertFollowEntity) -> Result<Uuid>;
    async fn unfollow(&self, user_id: Uuid, live_account_id: Uuid) -> Result<()>;
    async fn list_following_live_accounts(
        &self,
        user_id: Uuid,
        list_follows_filter: &ListFollowsFilter,
    ) -> Result<Vec<LiveAccountEntity>>;
    async fn find_live_account(
        &self,
        find_live_account_model: &FindLiveAccountModel,
    ) -> Result<LiveAccountEntity>;
}
