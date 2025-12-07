use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::invoices::InsertInvoiceEntity;

#[async_trait]
#[automock]
pub trait InvoiceRepository {
    async fn create_invoice(&self, invoice: InsertInvoiceEntity) -> Result<Uuid>;
    async fn mark_invoice_paid(&self, invoice_id: Uuid) -> Result<()>;
}
