use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::payments::NewPaymentEntity;

#[async_trait]
#[automock]
pub trait PaymentRepository {
    async fn record_payment(&self, payment: NewPaymentEntity) -> Result<Uuid>;
}
