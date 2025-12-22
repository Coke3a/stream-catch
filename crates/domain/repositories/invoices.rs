use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::invoices::{InsertInvoiceEntity, InvoiceEntity};

#[async_trait]
#[automock]
pub trait InvoiceRepository {
    async fn create_invoice(&self, invoice: InsertInvoiceEntity) -> Result<Uuid>;
    async fn mark_invoice_paid(&self, invoice_id: Uuid) -> Result<()>;
    async fn update_status_by_id(&self, invoice_id: Uuid, status: &str) -> Result<()>;
    async fn find_by_subscription_and_period_start(
        &self,
        subscription_id: Uuid,
        period_start: DateTime<Utc>,
    ) -> Result<Option<InvoiceEntity>>;
}
