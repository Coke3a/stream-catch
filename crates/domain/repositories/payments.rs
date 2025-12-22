use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::payments::NewPaymentEntity;
use crate::domain::value_objects::enums::payment_statuses::PaymentStatus;

#[async_trait]
#[automock]
pub trait PaymentRepository {
    async fn record_payment(&self, payment: NewPaymentEntity) -> Result<Uuid>;
    async fn update_status_by_provider_payment_id(
        &self,
        provider_payment_id: &str,
        status: PaymentStatus,
    ) -> Result<()>;
    async fn exists_by_provider_payment_id(&self, provider_payment_id: &str) -> Result<bool>;
}
