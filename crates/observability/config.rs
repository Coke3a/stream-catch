use std::env;
use tracing::Level;
use url::Url;

#[derive(Clone)]
pub(crate) struct ServiceContext {
    pub(crate) service_name: String,
    pub(crate) environment: String,
    pub(crate) component: String,
}

#[derive(Clone)]
pub(crate) struct DiscordConfig {
    pub(crate) webhook_url: Url,
    pub(crate) min_level: Level,
}

#[derive(Clone)]
pub(crate) struct ObservabilityConfig {
    pub(crate) service_context: ServiceContext,
    pub(crate) discord: Option<DiscordConfig>,
}

impl ObservabilityConfig {
    pub(crate) fn from_env(component: &str) -> Self {
        let component = component.trim().to_string();

        let service_name = env_string("SERVICE_NAME")
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| component.clone());

        let environment = env_string("APP_ENV")
            .filter(|v| !v.is_empty())
            .or_else(|| env_string("STAGE").filter(|v| !v.is_empty()))
            .unwrap_or_else(|| "unknown".to_string());

        let service_context = ServiceContext {
            service_name,
            environment,
            component,
        };

        let discord = match discord_from_env() {
            Ok(discord) => discord,
            Err(_) => None,
        };

        Self {
            service_context,
            discord,
        }
    }
}

fn discord_from_env() -> anyhow::Result<Option<DiscordConfig>> {
    let enabled = env_bool("DISCORD_NOTIFY_ENABLED").unwrap_or(true);

    let webhook_url = match env_string("DISCORD_WEBHOOK_URL").filter(|v| !v.is_empty()) {
        Some(raw) => Some(Url::parse(&raw)?),
        None => None,
    };

    if !enabled || webhook_url.is_none() {
        return Ok(None);
    }

    let min_level = env_string("DISCORD_NOTIFY_LEVEL")
        .and_then(|v| parse_level(&v))
        .unwrap_or(Level::ERROR);

    let webhook_url = webhook_url.expect("checked above");

    Ok(Some(DiscordConfig {
        webhook_url,
        min_level,
    }))
}

fn parse_level(input: &str) -> Option<Level> {
    match input.trim().to_ascii_lowercase().as_str() {
        "error" => Some(Level::ERROR),
        "warn" | "warning" => Some(Level::WARN),
        "info" => Some(Level::INFO),
        "debug" => Some(Level::DEBUG),
        "trace" => Some(Level::TRACE),
        _ => None,
    }
}

fn env_string(key: &str) -> Option<String> {
    env::var(key).ok()
}

fn env_bool(key: &str) -> Option<bool> {
    let raw = env::var(key).ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "t" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "f" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

