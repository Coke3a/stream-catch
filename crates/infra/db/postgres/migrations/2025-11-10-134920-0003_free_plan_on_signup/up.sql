CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER SET search_path = public
AS $$
DECLARE
  free_plan uuid;
  v_duration_days int;  -- renamed variable
BEGIN
  -- Ensure app_users row exists
  INSERT INTO public.app_users (id, status)
  VALUES (NEW.id, 'active')
  ON CONFLICT (id) DO NOTHING;

  -- Fetch free plan id from config
  SELECT (value->>'id')::uuid
  INTO free_plan
  FROM public.app_config
  WHERE key = 'free_plan_id';

  IF free_plan IS NULL THEN
    RETURN NEW;
  END IF;

  -- Only use active plans
  SELECT p.duration_days
  INTO v_duration_days
  FROM public.plans AS p
  WHERE p.id = free_plan
    AND p.is_active = true;

  IF v_duration_days IS NULL THEN
    RETURN NEW;
  END IF;

  -- Create subscription if none exists
  INSERT INTO public.subscriptions (
    user_id,
    plan_id,
    starts_at,
    ends_at,
    billing_mode,
    default_payment_method_id,
    cancel_at_period_end,
    canceled_at,
    provider_subscription_id,
    status
  )
  VALUES (
    NEW.id,
    free_plan,
    now(),
    now() + (v_duration_days || ' days')::interval,
    'recurring',
    NULL,
    false,
    NULL,
    NULL,
    'active'
  )
  ON CONFLICT DO NOTHING;

  RETURN NEW;
END;
$$;
