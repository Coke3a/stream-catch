use super::notifier::{NotificationEvent, NotificationProvider};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::SecondsFormat;
use reqwest::Client;
use serde_json::json;
use url::Url;

pub(crate) struct DiscordWebhookProvider {
    webhook_url: Url,
    client: Client,
}

impl DiscordWebhookProvider {
    pub(crate) fn new(webhook_url: Url) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .expect("reqwest client must build");

        Self {
            webhook_url,
            client,
        }
    }

    fn format_content(&self, event: &NotificationEvent) -> String {
        let mut lines = Vec::new();

        let level = event.level.as_str();
        lines.push(format!(
            "**{}** `{}` `{}` `{}`",
            event.service_name, event.environment, event.component, level
        ));

        lines.push(format!(
            "`{}` `{}`{}",
            event
                .timestamp
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            event.target,
            match (&event.file, event.line) {
                (Some(file), Some(line)) => format!(" `{}:{}`", file, line),
                _ => String::new(),
            }
        ));

        if let Some(message) = event.message.as_ref().filter(|m| !m.trim().is_empty()) {
            lines.push(format!("> {}", message.trim()));
        }

        if !event.spans.is_empty() {
            let span_chain = event
                .spans
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(" > ");
            lines.push(format!("spans: `{}`", span_chain));
        }

        if !event.fields.is_empty() {
            lines.push("fields:".to_string());
            for (k, v) in &event.fields {
                lines.push(format!("- `{}` = `{}`", k, v));
            }
        }

        for span in &event.spans {
            if span.fields.is_empty() {
                continue;
            }
            lines.push(format!("span `{}`:", span.name));
            for (k, v) in &span.fields {
                lines.push(format!("- `{}` = `{}`", k, v));
            }
        }

        truncate_for_discord(lines.join("\n"))
    }
}

#[async_trait]
impl NotificationProvider for DiscordWebhookProvider {
    async fn send(&self, event: &NotificationEvent) -> Result<()> {
        let content = self.format_content(event);

        let response = self
            .client
            .post(self.webhook_url.clone())
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(sanitize_reqwest_error)?;

        if response.status().is_success() {
            return Ok(());
        }

        Err(anyhow!(
            "discord webhook returned non-success status: {}",
            response.status()
        ))
    }

    fn provider_name(&self) -> &'static str {
        "discord"
    }
}

fn sanitize_reqwest_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        return anyhow!("discord webhook request timed out");
    }
    if error.is_connect() {
        return anyhow!("discord webhook connection failed");
    }
    anyhow!("discord webhook request failed")
}

fn truncate_for_discord(mut content: String) -> String {
    const LIMIT: usize = 2000;
    const SUFFIX: &str = "\n… (truncated)";

    if content.chars().count() <= LIMIT {
        return content;
    }

    let allowed = LIMIT.saturating_sub(SUFFIX.chars().count());
    let truncated: String = content.chars().take(allowed).collect();
    content.clear();
    content.push_str(&truncated);
    content.push_str(SUFFIX);
    content
}
