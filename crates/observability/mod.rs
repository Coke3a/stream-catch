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
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

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

    // Issue #1: Use EnvFilter (RUST_LOG) with a safe default to avoid forcing TRACE in production.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Default `SystemTime` formatter prints RFC3339 in UTC (`...Z`).
    // Use local time so `TZ=Asia/Bangkok` shows `+07:00` in logs.
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(notify_layer)
        .with(env_filter)
        .try_init()?;

    // Issue #2: Make optional Discord sink misconfiguration visible during startup.
    for warning in &config.warnings {
        warn!(
            service = %config.service_context.service_name,
            environment = %config.service_context.environment,
            component = %config.service_context.component,
            warning = %warning,
            "Observability config warning"
        );
    }

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

pub async fn send_discord_webhook(webhook_url: Url, content: String) -> Result<()> {
    discord::send_discord_webhook(webhook_url, content).await
}
