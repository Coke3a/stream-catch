use anyhow::Result;

use crate::domain::value_objects::oneliverec_webhook::OneLiveRecWebhook;

pub struct OneLiveRecWebhookUseCase;

impl OneLiveRecWebhookUseCase {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle_webhook(&self, payload: OneLiveRecWebhook) -> Result<()> {
        let _payload = payload;
        unimplemented!()
    }
}
