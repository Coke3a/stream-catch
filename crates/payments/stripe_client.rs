use std::collections::HashMap;

use anyhow::Result;
use hmac::{Hmac, Mac};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Minimal Stripe client built on reqwest.
pub struct StripeClient {
    http: reqwest::Client,
    secret_key: String,
    webhook_secret: String,
    success_url: String,
    cancel_url: String,
}

#[derive(Debug, Deserialize)]
pub struct StripeEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: StripeEventData,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct StripeCheckoutSession {
    pub id: Option<String>,
    pub mode: Option<String>,
    pub subscription: Option<String>,
    pub customer: Option<String>,
    pub payment_intent: Option<String>,
    pub amount_total: Option<i64>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct StripeSubscription {
    pub current_period_start: Option<i64>,
    pub current_period_end: Option<i64>,
    pub billing_cycle_anchor: Option<i64>,
    #[serde(default)]
    pub items: StripeSubscriptionItems,
}

#[derive(Debug, Deserialize, Default)]
pub struct StripeSubscriptionItems {
    pub data: Vec<StripeSubscriptionItem>,
}

#[derive(Debug, Deserialize)]
pub struct StripeSubscriptionItem {
    pub current_period_start: Option<i64>,
    pub current_period_end: Option<i64>,
}

impl StripeSubscription {
    /// Returns the subscription period start timestamp, falling back to the first item
    /// or the billing cycle anchor when the top-level field is absent.
    pub fn period_start(&self) -> Option<i64> {
        self.current_period_start
            .or_else(|| {
                self.items
                    .data
                    .first()
                    .and_then(|item| item.current_period_start)
            })
            .or(self.billing_cycle_anchor)
    }

    /// Returns the subscription period end timestamp, falling back to the first item when needed.
    pub fn period_end(&self) -> Option<i64> {
        self.current_period_end.or_else(|| {
            self.items
                .data
                .first()
                .and_then(|item| item.current_period_end)
        })
    }
}

impl StripeClient {
    pub fn new(
        secret_key: String,
        webhook_secret: String,
        success_url: String,
        cancel_url: String,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            secret_key,
            webhook_secret,
            success_url,
            cancel_url,
        }
    }

    /// Creates or reuses a Stripe customer for the given email/user.
    pub async fn create_customer(&self, email: &str, user_id: Uuid) -> Result<String> {
        // See Stripe customer docs: https://stripe.com/docs/api/customers/create
        let body = [
            ("email", email.to_string()),
            ("metadata[user_id]", user_id.to_string()),
        ];

        let resp = self
            .http
            .post("https://api.stripe.com/v1/customers")
            .header(AUTHORIZATION, format!("Bearer {}", self.secret_key))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&body)
            .send()
            .await?
            .error_for_status()?;

        #[derive(Deserialize)]
        struct CustomerResp {
            id: String,
        }

        let parsed: CustomerResp = resp.json().await?;
        Ok(parsed.id)
    }

    /// Creates a Checkout Session and returns its URL.
    pub async fn create_checkout_session(
        &self,
        price_id: &str,
        mode: &str,
        customer_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        // Stripe Checkout docs:
        // https://stripe.com/docs/payments/checkout
        let mut body: Vec<(String, String)> = vec![
            ("mode".to_string(), mode.to_string()),
            ("line_items[0][price]".to_string(), price_id.to_string()),
            ("line_items[0][quantity]".to_string(), "1".to_string()),
            ("success_url".to_string(), self.success_url.clone()),
            ("cancel_url".to_string(), self.cancel_url.clone()),
        ];

        // for (idx, pm) in payment_method_types.iter().enumerate() {
        //     body.push((format!("payment_method_types[{}]", idx), pm.clone()));
        // }

        if let Some(customer) = customer_id {
            body.push(("customer".to_string(), customer));
        }

        for (key, value) in metadata {
            body.push((format!("metadata[{}]", key), value));
        }

        let resp = self
            .http
            .post("https://api.stripe.com/v1/checkout/sessions")
            .header(AUTHORIZATION, format!("Bearer {}", self.secret_key))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&body)
            .send()
            .await?
            .error_for_status()?;

        #[derive(Deserialize)]
        struct CheckoutResp {
            url: Option<String>,
        }

        let parsed: CheckoutResp = resp.json().await?;
        parsed
            .url
            .ok_or_else(|| anyhow::anyhow!("Stripe Checkout session URL is missing"))
    }

    /// Marks a Stripe subscription to cancel at period end.
    pub async fn cancel_subscription(&self, provider_subscription_id: &str) -> Result<()> {
        // https://stripe.com/docs/api/subscriptions/cancel#cancel_subscription-at_period_end
        let body = [("cancel_at_period_end", "true".to_string())];
        self.http
            .post(format!(
                "https://api.stripe.com/v1/subscriptions/{}",
                provider_subscription_id
            ))
            .header(AUTHORIZATION, format!("Bearer {}", self.secret_key))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Verifies the webhook signature. https://stripe.com/docs/webhooks/signatures
    pub fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<StripeEvent> {
        let mut timestamp: Option<String> = None;
        let mut signature: Option<String> = None;

        for part in signature_header.split(',') {
            if let Some(rest) = part.strip_prefix("t=") {
                timestamp = Some(rest.to_string());
            } else if let Some(rest) = part.strip_prefix("v1=") {
                signature = Some(rest.to_string());
            }
        }

        let timestamp =
            timestamp.ok_or_else(|| anyhow::anyhow!("missing timestamp in stripe-signature"))?;
        let signature =
            signature.ok_or_else(|| anyhow::anyhow!("missing v1 in stripe-signature"))?;

        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
        let mut mac = HmacSha256::new_from_slice(self.webhook_secret.as_bytes())?;
        mac.update(signed_payload.as_bytes());
        let expected = mac.finalize().into_bytes();
        let provided = hex::decode(signature)?;

        if expected[..] != provided[..] {
            anyhow::bail!("invalid webhook signature");
        }

        let event: StripeEvent = serde_json::from_slice(payload)?;
        Ok(event)
    }

    pub fn extract_checkout_session(event: &StripeEvent) -> Option<StripeCheckoutSession> {
        serde_json::from_value(event.data.object.clone()).ok()
    }

    pub async fn retrieve_subscription(&self, subscription_id: &str) -> Result<StripeSubscription> {
        // https://stripe.com/docs/api/subscriptions/retrieve
        let resp = self
            .http
            .get(format!(
                "https://api.stripe.com/v1/subscriptions/{}",
                subscription_id
            ))
            .header(AUTHORIZATION, format!("Bearer {}", self.secret_key))
            .send()
            .await?
            .error_for_status()?;

        let subscription: StripeSubscription = resp.json().await?;
        Ok(subscription)
    }
}
