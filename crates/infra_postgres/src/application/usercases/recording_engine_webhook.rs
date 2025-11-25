use anyhow::Result;

use crate::domain::value_objects::recording_engine_webhook::RecordingEngineWebhook;

pub struct RecordingEngineWebhookUseCase;

impl RecordingEngineWebhookUseCase {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle_webhook(&self, payload: RecordingEngineWebhook) -> Result<()> {
        let _payload = payload;
        unimplemented!()
    }
}
