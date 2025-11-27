-- MVP1 – Web‑centric Video Recording & Delivery
-- Supabase‑first Postgres schema (aligned with OneLiveRec spec)
--
-- Notes:
--  - Uses Supabase Auth (auth.users) as the source of user IDs.
--  - "app_users" stores per‑app profile data and references auth.users(id).
--  - All user_id columns are UUIDs referencing app_users(id).
--  - Includes a simple jobs queue and optional deliveries table.
--  - Designed to be pasted into Supabase SQL editor in a fresh project.

-- =========================
-- 0. Extensions
-- =========================

-- UUID generation for primary keys
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Case‑insensitive text (not used directly yet, but handy for future email/ID columns)
CREATE EXTENSION IF NOT EXISTS citext;

-- Needed for the EXCLUDE constraint that prevents overlapping subscriptions
CREATE EXTENSION IF NOT EXISTS btree_gist;


-- =========================
-- 1. Core user profile
-- =========================

-- Supabase Auth already manages authentication and password reset in auth.users.
-- This table stores per‑application profile data.

CREATE TABLE "app_users" (
  "id" uuid PRIMARY KEY,
  "status" TEXT NOT NULL CHECK ("status" IN ('active','blocked','inactive')) DEFAULT 'active',
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);


-- =========================
-- 2. Plans & subscriptions
-- =========================

CREATE TABLE "plans" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "name" TEXT,
  "price_minor" INT NOT NULL CHECK ("price_minor" >= 0),
  "duration_days" INT NOT NULL CHECK ("duration_days" > 0),
  "features" JSONB NOT NULL DEFAULT '{}'::jsonb,
  "is_active" BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE "subscriptions" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "user_id" uuid NOT NULL,
  "plan_id" uuid NOT NULL,
  "starts_at" timestamptz NOT NULL,
  "ends_at" timestamptz NOT NULL,
  "billing_mode" TEXT NOT NULL CHECK ("billing_mode" IN ('recurring','manual')) DEFAULT 'recurring',
  "default_payment_method_id" uuid,
  "cancel_at_period_end" BOOLEAN NOT NULL DEFAULT false,
  "canceled_at" timestamptz,
  "status" TEXT NOT NULL CHECK ("status" IN ('active','past_due','canceled','expired')),
  "created_at" timestamptz NOT NULL DEFAULT now(),
  CHECK ("starts_at" < "ends_at")
);


-- =========================
-- 3. Payments & invoices (Stripe integration)
-- =========================

CREATE TABLE "payment_provider_customers" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "user_id" uuid NOT NULL,
  "provider" TEXT NOT NULL,
  "customer_ref" TEXT NOT NULL,
  "metadata" JSONB NOT NULL DEFAULT '{}'::jsonb,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE "payment_methods" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "user_id" uuid NOT NULL,
  "provider" TEXT NOT NULL,
  "method_type" TEXT NOT NULL CHECK ("method_type" IN ('card','promptpay')),
  "pm_ref" TEXT NOT NULL,
  "brand" TEXT,
  "last4" TEXT,
  "exp_month" INT,
  "exp_year" INT,
  "status" TEXT NOT NULL CHECK ("status" IN ('active','inactive','expired')) DEFAULT 'active',
  "is_default" BOOLEAN NOT NULL DEFAULT false,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE "invoices" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "user_id" uuid NOT NULL,
  "subscription_id" uuid,
  "plan_id" uuid NOT NULL,
  "amount_minor" INT NOT NULL CHECK ("amount_minor" >= 0),
  "period_start" timestamptz NOT NULL,
  "period_end" timestamptz NOT NULL,
  "due_at" timestamptz NOT NULL,
  "status" TEXT NOT NULL CHECK ("status" IN ('pending','paid','failed','void','past_due')),
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "paid_at" timestamptz
);

CREATE TABLE "payments" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "invoice_id" uuid NOT NULL,
  "user_id" uuid NOT NULL,
  "provider" TEXT NOT NULL,
  "method_type" TEXT NOT NULL CHECK ("method_type" IN ('card','promptpay')),
  "payment_method_id" uuid,
  "amount_minor" INT NOT NULL CHECK ("amount_minor" >= 0),
  "status" TEXT NOT NULL CHECK ("status" IN ('requires_action','processing','succeeded','failed','canceled')),
  "provider_payment_id" TEXT,
  "provider_session_ref" TEXT,
  "error" TEXT,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);


-- =========================
-- 4. Live accounts, follows, recordings
-- =========================

CREATE TABLE "live_accounts" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "platform" TEXT NOT NULL,
  "account_id" TEXT NOT NULL,
  "canonical_url" TEXT NOT NULL,
  "status" TEXT NOT NULL CHECK ("status" IN ('synced','unsynced','error')) DEFAULT 'unsynced',
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE "follows" (
  "user_id" uuid NOT NULL,
  "live_account_id" uuid NOT NULL,
  "status" TEXT NOT NULL CHECK ("status" IN ('active','inactive','temporary_inactive')) DEFAULT 'active',
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY ("user_id", "live_account_id")
);

CREATE TABLE "recordings" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "live_account_id" uuid NOT NULL,
  "recording_key" TEXT UNIQUE,
  "title" TEXT,
  "started_at" timestamptz NOT NULL,
  "ended_at" timestamptz,
  "duration_sec" INT,
  "size_bytes" BIGINT,
  "storage_path" TEXT,
  "storage_temp_path" TEXT,
  "status" TEXT NOT NULL CHECK ("status" IN ('live_recording','live_end','waiting_upload','uploading','ready','failed')) DEFAULT 'live_recording',
  "poster_storage_path" TEXT,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);


-- =========================
-- 5. Deliveries & jobs
-- =========================

CREATE TABLE "deliveries" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "recording_id" uuid NOT NULL,
  "user_id" uuid NOT NULL,
  "via" TEXT NOT NULL CHECK ("via" IN ('web_notify','email','telegram')),
  "delivered_at" timestamptz,
  "status" TEXT NOT NULL CHECK ("status" IN ('queued','sent','failed')) DEFAULT 'queued',
  "error" TEXT
);

CREATE TABLE "jobs" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "type" TEXT NOT NULL CHECK ("type" IN ('RecordingUpload', 'NotifyReady')),
  "payload" JSONB NOT NULL,
  "run_at" timestamptz NOT NULL DEFAULT now(),
  "attempts" INT NOT NULL DEFAULT 0,
  "locked_at" timestamptz,
  "locked_by" TEXT,
  "error" TEXT,
  "status" TEXT NOT NULL CHECK ("status" IN ('queued','running','done','failed','dead')) DEFAULT 'queued',
  "created_at" timestamptz NOT NULL DEFAULT now()
);


-- =========================
-- 6. Indexes
-- =========================

-- Subscriptions: avoid accidental duplicates per user/plan/period start
CREATE UNIQUE INDEX "subscriptions_user_plan_start_idx"
  ON "subscriptions" ("user_id", "plan_id", "starts_at");

-- Provider customers (e.g. Stripe customer IDs)
CREATE UNIQUE INDEX "payment_provider_customers_user_provider_uidx"
  ON "payment_provider_customers" ("user_id", "provider");

CREATE UNIQUE INDEX "payment_provider_customers_provider_ref_uidx"
  ON "payment_provider_customers" ("provider", "customer_ref");

-- Payment methods
CREATE UNIQUE INDEX "payment_methods_provider_pm_ref_uidx"
  ON "payment_methods" ("provider", "pm_ref");

CREATE INDEX "payment_methods_user_idx"
  ON "payment_methods" ("user_id");

-- Invoices
CREATE UNIQUE INDEX "invoices_subscription_period_start_uidx"
  ON "invoices" ("subscription_id", "period_start");

CREATE INDEX "invoices_user_status_idx"
  ON "invoices" ("user_id", "status");

CREATE INDEX "invoices_due_idx"
  ON "invoices" ("status", "due_at");

-- Payments
CREATE INDEX "payments_invoice_idx"
  ON "payments" ("invoice_id");

-- Live accounts
CREATE UNIQUE INDEX "live_accounts_platform_account_uidx"
  ON "live_accounts" ("platform", "account_id");

-- Deliveries: one row per recording/user/channel
CREATE UNIQUE INDEX "deliveries_recording_user_via_uidx"
  ON "deliveries" ("recording_id", "user_id", "via");

-- Jobs queue lookup
CREATE INDEX "jobs_status_run_at_idx"
  ON "jobs" ("status", "run_at");


-- Optional: prevent overlapping subscription periods for the same user
-- This matches the spec description "Unique index prevents overlapping subscriptions".
ALTER TABLE "subscriptions"
  ADD CONSTRAINT "subscriptions_user_timerange_excl"
  EXCLUDE USING gist (
    "user_id" WITH =,
    tstzrange("starts_at", "ends_at", '[)') WITH &&
  );


-- =========================
-- 7. Foreign keys
-- =========================

-- Link app_users to Supabase Auth users
ALTER TABLE "app_users"
  ADD CONSTRAINT "app_users_auth_users_fkey"
  FOREIGN KEY ("id") REFERENCES auth.users("id") ON DELETE CASCADE;

-- Subscriptions
ALTER TABLE "subscriptions"
  ADD FOREIGN KEY ("user_id") REFERENCES "app_users"("id");

ALTER TABLE "subscriptions"
  ADD FOREIGN KEY ("plan_id") REFERENCES "plans"("id");

ALTER TABLE "subscriptions"
  ADD FOREIGN KEY ("default_payment_method_id") REFERENCES "payment_methods"("id");

-- Payment provider customers
ALTER TABLE "payment_provider_customers"
  ADD FOREIGN KEY ("user_id") REFERENCES "app_users"("id");

-- Payment methods
ALTER TABLE "payment_methods"
  ADD FOREIGN KEY ("user_id") REFERENCES "app_users"("id");

-- Invoices
ALTER TABLE "invoices"
  ADD FOREIGN KEY ("user_id") REFERENCES "app_users"("id");

ALTER TABLE "invoices"
  ADD FOREIGN KEY ("subscription_id") REFERENCES "subscriptions"("id");

ALTER TABLE "invoices"
  ADD FOREIGN KEY ("plan_id") REFERENCES "plans"("id");

-- Payments
ALTER TABLE "payments"
  ADD FOREIGN KEY ("invoice_id") REFERENCES "invoices"("id");

ALTER TABLE "payments"
  ADD FOREIGN KEY ("user_id") REFERENCES "app_users"("id");

ALTER TABLE "payments"
  ADD FOREIGN KEY ("payment_method_id") REFERENCES "payment_methods"("id");

-- Follows
ALTER TABLE "follows"
  ADD FOREIGN KEY ("user_id") REFERENCES "app_users"("id");

ALTER TABLE "follows"
  ADD FOREIGN KEY ("live_account_id") REFERENCES "live_accounts"("id");

-- Recordings
ALTER TABLE "recordings"
  ADD FOREIGN KEY ("live_account_id") REFERENCES "live_accounts"("id");

-- Deliveries
ALTER TABLE "deliveries"
  ADD FOREIGN KEY ("recording_id") REFERENCES "recordings"("id");

ALTER TABLE "deliveries"
  ADD FOREIGN KEY ("user_id") REFERENCES "app_users"("id");


-- trigger when create user in subapase
create or replace function public.handle_new_user()
returns trigger
language plpgsql
security definer set search_path = public
as $$
begin
insert into public.app_users (id, status)
values (new.id, 'active');
return new;
end;
$$;

drop trigger if exists on_auth_user_created on auth.users;

create trigger on_auth_user_created
after insert on auth.users
for each row execute procedure public.handle_new_user();
