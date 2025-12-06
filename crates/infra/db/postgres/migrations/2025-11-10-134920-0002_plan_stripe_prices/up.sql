-- Stripe price mapping per plan
-- Checkout session creation will read these price IDs.
--   - stripe_price_recurring: recurring subscription price (card only)
--   - stripe_price_one_time_card: one-time payment price (card)
--   - stripe_price_one_time_promptpay: one-time payment price (PromptPay)
ALTER TABLE public.plans
  ADD COLUMN IF NOT EXISTS stripe_price_recurring TEXT,
  ADD COLUMN IF NOT EXISTS stripe_price_one_time_card TEXT,
  ADD COLUMN IF NOT EXISTS stripe_price_one_time_promptpay TEXT;

-- Store provider subscription identifiers (e.g., Stripe subscription id)
ALTER TABLE public.subscriptions
  ADD COLUMN IF NOT EXISTS provider_subscription_id TEXT;
