use super::config::ServiceContext;
use super::notifier::{NotificationEvent, Notifier, SpanSummary};
use chrono::Utc;
use std::collections::BTreeMap;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

#[derive(Clone)]
pub(crate) struct ErrorNotifyLayer {
    notifier: Notifier,
    service_context: ServiceContext,
    min_level: Level,
}

impl ErrorNotifyLayer {
    pub(crate) fn new(notifier: Notifier, service_context: ServiceContext, min_level: Level) -> Self {
        Self {
            notifier,
            service_context,
            min_level,
        }
    }
}

#[derive(Default)]
struct FieldMapVisitor {
    values: BTreeMap<String, String>,
}

impl Visit for FieldMapVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.values
            .insert(field.name().to_string(), redact(field.name(), format!("{value:?}")));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(field.name().to_string(), redact(field.name(), value.to_string()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(field.name().to_string(), redact(field.name(), value.to_string()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(field.name().to_string(), redact(field.name(), value.to_string()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(field.name().to_string(), redact(field.name(), value.to_string()));
    }
}

#[derive(Default)]
struct SpanFieldMap {
    values: BTreeMap<String, String>,
}

impl<S> Layer<S> for ErrorNotifyLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &tracing::span::Id, ctx: Context<'_, S>) {
        let mut visitor = FieldMapVisitor::default();
        attrs.record(&mut visitor);

        if visitor.values.is_empty() {
            return;
        }

        if let Some(span) = ctx.span(id) {
            span.extensions_mut()
                .insert(SpanFieldMap { values: visitor.values });
        }
    }

    fn on_record(&self, id: &tracing::span::Id, values: &tracing::span::Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };

        let mut visitor = FieldMapVisitor::default();
        values.record(&mut visitor);

        if visitor.values.is_empty() {
            return;
        }

        let mut extensions = span.extensions_mut();
        let fields = extensions.get_mut::<SpanFieldMap>();
        match fields {
            Some(existing) => {
                existing.values.extend(visitor.values);
            }
            None => {
                extensions.insert(SpanFieldMap { values: visitor.values });
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if *event.metadata().level() < self.min_level {
            return;
        }

        let mut visitor = FieldMapVisitor::default();
        event.record(&mut visitor);

        let mut message = None;
        if let Some(raw) = visitor.values.remove("message") {
            message = Some(unquote_debug_string(&raw));
        }

        let spans = ctx
            .event_span(event)
            .map(|span| {
                span.scope()
                    .from_root()
                    .filter_map(|s| {
                        let name = s.metadata().name().to_string();
                        let fields = s
                            .extensions()
                            .get::<SpanFieldMap>()
                            .map(|m| m.values.clone())
                            .unwrap_or_default();
                        Some(SpanSummary { name, fields })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let notification = NotificationEvent {
            level: *event.metadata().level(),
            timestamp: Utc::now(),
            service_name: self.service_context.service_name.clone(),
            environment: self.service_context.environment.clone(),
            component: self.service_context.component.clone(),
            target: event.metadata().target().to_string(),
            file: event.metadata().file().map(|f| f.to_string()),
            line: event.metadata().line(),
            message,
            fields: visitor.values,
            spans,
        };

        self.notifier.try_notify(notification);
    }
}

fn unquote_debug_string(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    trimmed.to_string()
}

fn redact(field_name: &str, value: String) -> String {
    if is_sensitive_key(field_name) {
        return "[REDACTED]".to_string();
    }
    value
}

fn is_sensitive_key(field_name: &str) -> bool {
    let field = field_name.to_ascii_lowercase();
    field.contains("webhook")
        || field.contains("secret")
        || field.contains("password")
        || field.contains("token")
        || field.contains("authorization")
}
