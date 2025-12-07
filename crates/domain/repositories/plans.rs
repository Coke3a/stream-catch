use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::plans::PlanEntity;

#[async_trait]
#[automock]
pub trait PlanRepository {
    async fn find_by_id(&self, plan_id: Uuid) -> Result<PlanEntity>;
    async fn find_active_plan_by_id(&self, plan_id: Uuid) -> Result<PlanEntity>;
    async fn list_active_plans(&self) -> Result<Vec<PlanEntity>>;
}
