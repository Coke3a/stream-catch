-- Ensure subscriptions table has provider_subscription_id for external billing (e.g., Stripe)
ALTER TABLE public.subscriptions
  ADD COLUMN IF NOT EXISTS provider_subscription_id TEXT;
