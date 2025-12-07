use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

#[async_trait]
#[automock]
pub trait PaymentProviderCustomerRepository {
    async fn find_or_create_stripe_customer_id(&self, user_id: Uuid, email: &str)
    -> Result<String>;

    async fn upsert_customer_ref(
        &self,
        user_id: Uuid,
        provider: &str,
        customer_ref: &str,
    ) -> Result<()>;
}
