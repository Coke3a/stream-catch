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
    /// Warnings captured during config parsing so they can be logged after tracing is initialized.
    pub(crate) warnings: Vec<String>,
}

impl ObservabilityConfig {
    pub(crate) fn from_env(component: &str) -> Self {
        let component = component.trim().to_string();

        let service_name = env_string("SERVICE_NAME")
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| component.clone());

        let environment = env_string("STAGE").filter(|v| !v.is_empty())
            .unwrap_or_else(|| "unknown".to_string());

        let service_context = ServiceContext {
            service_name,
            environment,
            component,
        };

        // Collect parsing warnings instead of silently disabling optional sinks.
        let (discord, warnings) = discord_from_env();

        Self {
            service_context,
            discord,
            warnings,
        }
    }
}

fn discord_from_env() -> (Option<DiscordConfig>, Vec<String>) {
    let mut warnings = Vec::new();

    let enabled = env_bool("DISCORD_NOTIFY_ENABLED").unwrap_or(true);

    let webhook_url_raw = env_string("DISCORD_WEBHOOK_URL").filter(|v| !v.is_empty());
    let webhook_url = if !enabled {
        None
    } else if let Some(raw) = webhook_url_raw.as_deref() {
        match Url::parse(raw) {
            Ok(url) => Some(url),
            Err(err) => {
                // Do not include the raw URL in logs (webhook URLs contain secrets).
                warnings.push(format!(
                    "DISCORD_WEBHOOK_URL is set but invalid; Discord notifications disabled (parse error: {err})"
                ));
                None
            }
        }
    } else {
        None
    };

    if !enabled || webhook_url.is_none() {
        return (None, warnings);
    }

    let min_level = match env_string("DISCORD_NOTIFY_LEVEL") {
        Some(raw) if !raw.trim().is_empty() => match parse_level(&raw) {
            Some(level) => level,
            None => {
                // Make level misconfiguration visible during startup while staying non-fatal.
                warnings.push(format!(
                    "DISCORD_NOTIFY_LEVEL is invalid (value: {raw}); defaulting to ERROR"
                ));
                Level::ERROR
            }
        },
        _ => Level::ERROR,
    };

    let webhook_url = webhook_url.expect("checked above");

    (
        Some(DiscordConfig {
        webhook_url,
        min_level,
        }),
        warnings,
    )
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
