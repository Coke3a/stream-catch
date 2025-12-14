use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::Level;
use tracing::warn;

#[derive(Clone, Debug)]
pub(crate) struct SpanSummary {
    pub(crate) name: String,
    pub(crate) fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(crate) struct NotificationEvent {
    pub(crate) level: Level,
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) service_name: String,
    pub(crate) environment: String,
    pub(crate) component: String,
    pub(crate) target: String,
    pub(crate) file: Option<String>,
    pub(crate) line: Option<u32>,
    pub(crate) message: Option<String>,
    pub(crate) fields: BTreeMap<String, String>,
    pub(crate) spans: Vec<SpanSummary>,
}

#[async_trait]
pub(crate) trait NotificationProvider: Send + Sync {
    async fn send(&self, event: &NotificationEvent) -> Result<()>;
    fn provider_name(&self) -> &'static str;
}

#[derive(Clone)]
pub(crate) struct Notifier {
    tx: mpsc::Sender<NotificationEvent>,
}

impl Notifier {
    pub(crate) fn new(providers: Vec<Arc<dyn NotificationProvider>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<NotificationEvent>(256);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                for provider in &providers {
                    if let Err(error) = provider.send(&event).await {
                        warn!(
                            provider = provider.provider_name(),
                            error = %error,
                            "Notification provider failed"
                        );
                    }
                }
            }
        });

        Self { tx }
    }

    pub(crate) fn try_notify(&self, event: NotificationEvent) {
        match self.tx.try_send(event) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!("Notification queue full; dropping event");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!("Notification queue closed; dropping event");
            }
        }
    }
}
