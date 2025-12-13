mod config;
mod discord;
mod layer;
mod notifier;

use anyhow::Result;
use config::ObservabilityConfig;
use discord::DiscordWebhookProvider;
use layer::ErrorNotifyLayer;
use notifier::Notifier;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

pub fn init_observability(component: &str) -> Result<()> {
    let config = ObservabilityConfig::from_env(component);

    let notify_layer = config.discord.as_ref().map(|discord| {
        let notifier = Notifier::new(vec![Arc::new(DiscordWebhookProvider::new(
            discord.webhook_url.clone(),
        ))]);

        ErrorNotifyLayer::new(notifier, config.service_context.clone(), discord.min_level)
            .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
                discord.min_level,
            ))
    });

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .finish()
        .with(notify_layer)
        .try_init()?;

    if config.discord.is_some() {
        info!(
            service = %config.service_context.service_name,
            environment = %config.service_context.environment,
            component = %config.service_context.component,
            "Discord error notifications enabled"
        );
    } else {
        info!(
            service = %config.service_context.service_name,
            environment = %config.service_context.environment,
            component = %config.service_context.component,
            "Discord error notifications disabled"
        );
    }

    Ok(())
}
