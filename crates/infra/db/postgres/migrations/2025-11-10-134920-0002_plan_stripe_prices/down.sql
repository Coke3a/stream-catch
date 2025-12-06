ALTER TABLE public.plans
  DROP COLUMN IF EXISTS stripe_price_recurring,
  DROP COLUMN IF EXISTS stripe_price_one_time_card,
  DROP COLUMN IF EXISTS stripe_price_one_time_promptpay;

ALTER TABLE public.subscriptions
  DROP COLUMN IF EXISTS provider_subscription_id;
