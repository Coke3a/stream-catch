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
  "billing_mode" TEXT NOT NULL CHECK ("billing_mode" IN ('recurring','one_time')) DEFAULT 'recurring',
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



-- =========================================================
-- RLS POLICIES FOR STREAMCATCH / SUPABASE SCHEMA
-- =========================================================
-- หมายเหตุ:
-- - ใช้ auth.uid() เป็นตัวแทน user ปัจจุบัน
-- - service_role (backend) bypass RLS ได้โดยอัตโนมัติ
-- - สคริปต์นี้ใช้ DROP POLICY IF EXISTS เพื่อให้รันซ้ำได้
-- =========================================================

-- =========================
-- 1. app_users
--   - ผู้ใช้เห็น/แก้ไขข้อมูล profile ของตัวเองเท่านั้น
--   - insert มาจาก trigger handle_new_user() ผ่าน auth.users
-- =========================

ALTER TABLE public.app_users ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "user can view own profile" ON public.app_users;
CREATE POLICY "user can view own profile"
  ON public.app_users
  FOR SELECT
  USING (id = auth.uid());

DROP POLICY IF EXISTS "user can update own profile" ON public.app_users;
CREATE POLICY "user can update own profile"
  ON public.app_users
  FOR UPDATE
  USING (id = auth.uid());


-- =========================
-- 2. plans
--   - ทุกคนอ่านได้ (ใช้โชว์หน้าราคา / plans)
-- =========================

ALTER TABLE public.plans ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "plans readable by everyone" ON public.plans;
CREATE POLICY "plans readable by everyone"
  ON public.plans
  FOR SELECT
  USING (true);


-- =========================
-- 3. subscriptions
--   - ผู้ใช้เห็น subscription ของตัวเองเท่านั้น
--   - การสร้าง/แก้ไข แนะนำให้ทำผ่าน backend (service_role)
--     ถ้าอยากให้ client เขียนเอง ค่อยเพิ่ม INSERT/UPDATE policies
-- =========================

ALTER TABLE public.subscriptions ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "users read own subscriptions" ON public.subscriptions;
CREATE POLICY "users read own subscriptions"
  ON public.subscriptions
  FOR SELECT
  USING (user_id = auth.uid());


-- =========================
-- 4. payment_provider_customers
--   - ผู้ใช้เห็น customer record ของตัวเองเท่านั้น
-- =========================

ALTER TABLE public.payment_provider_customers ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "users read own provider customers" ON public.payment_provider_customers;
CREATE POLICY "users read own provider customers"
  ON public.payment_provider_customers
  FOR SELECT
  USING (user_id = auth.uid());


-- =========================
-- 5. payment_methods
--   - ผู้ใช้เห็น payment methods ของตัวเองเท่านั้น
--   - การสร้าง/แก้ไข แนะนำให้ทำผ่าน backend + service_role
-- =========================

ALTER TABLE public.payment_methods ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "users read own payment methods" ON public.payment_methods;
CREATE POLICY "users read own payment methods"
  ON public.payment_methods
  FOR SELECT
  USING (user_id = auth.uid());


-- =========================
-- 6. invoices
--   - ผู้ใช้เห็น invoice ของตัวเองเท่านั้น
-- =========================

ALTER TABLE public.invoices ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "users read own invoices" ON public.invoices;
CREATE POLICY "users read own invoices"
  ON public.invoices
  FOR SELECT
  USING (user_id = auth.uid());


-- =========================
-- 7. payments
--   - ผู้ใช้เห็น payment records ที่ของตัวเองเท่านั้น
-- =========================

ALTER TABLE public.payments ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "users read own payments" ON public.payments;
CREATE POLICY "users read own payments"
  ON public.payments
  FOR SELECT
  USING (user_id = auth.uid());


-- =========================
-- 8. live_accounts
--   - ช่อง live เป็นข้อมูลกึ่งสาธารณะ → ให้ทุกคนอ่านได้
-- =========================

ALTER TABLE public.live_accounts ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "everyone can read live accounts" ON public.live_accounts;
CREATE POLICY "everyone can read live accounts"
  ON public.live_accounts
  FOR SELECT
  USING (true);


-- =========================
-- 9. follows
--   - ผู้ใช้เห็น/จัดการ follow ของตัวเองเท่านั้น
-- =========================

ALTER TABLE public.follows ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "users read own follows" ON public.follows;
CREATE POLICY "users read own follows"
  ON public.follows
  FOR SELECT
  USING (user_id = auth.uid());

DROP POLICY IF EXISTS "users insert own follows" ON public.follows;
CREATE POLICY "users insert own follows"
  ON public.follows
  FOR INSERT
  WITH CHECK (user_id = auth.uid());

DROP POLICY IF EXISTS "users update own follows" ON public.follows;
CREATE POLICY "users update own follows"
  ON public.follows
  FOR UPDATE
  USING (user_id = auth.uid())
  WITH CHECK (user_id = auth.uid());


-- =========================
-- 10. recordings
--   - เงื่อนไขตามที่คุณขอ:
--     * user เห็นได้เฉพาะ recordings ที่:
--         - live_account นั้น user ได้ follow (ตามตาราง follows)
--         - AND status = 'ready'
--     * ห้ามเห็นรายการที่ยัง upload, failed, live ฯลฯ
--   - Backend (service_role) ยังเห็นได้หมดเหมือนเดิม (bypass RLS)
-- =========================

ALTER TABLE public.recordings ENABLE ROW LEVEL SECURITY;

-- ลบ policy เก่า (ถ้ามี) เพื่อกันชนกัน
DROP POLICY IF EXISTS "users read recordings of followed accounts" ON public.recordings;
DROP POLICY IF EXISTS "users read ready recordings of followed accounts" ON public.recordings;

CREATE POLICY "users read ready recordings of followed accounts"
  ON public.recordings
  FOR SELECT
  USING (
    status = 'ready'
    AND EXISTS (
      SELECT 1
      FROM public.follows
      WHERE follows.live_account_id = recordings.live_account_id
        AND follows.user_id = auth.uid()
        AND follows.status = 'active'
    )
  );


-- =========================
-- 11. deliveries
--   - ผู้ใช้เห็น delivery logs ที่ของตัวเองเท่านั้น
-- =========================

ALTER TABLE public.deliveries ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "user sees own deliveries" ON public.deliveries;
CREATE POLICY "user sees own deliveries"
  ON public.deliveries
  FOR SELECT
  USING (user_id = auth.uid());


-- =========================
-- 12. jobs
--   - ใช้เป็น internal queue เท่านั้น
--   - ปิดไม่ให้ anon/authenticated อ่าน/เขียนโดยตรง
--   - backend ใช้ service_role (bypass RLS) ในการจัดการ
-- =========================

ALTER TABLE public.jobs ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "deny read for normal users" ON public.jobs;
CREATE POLICY "deny read for normal users"
  ON public.jobs
  FOR SELECT
  USING (false);

DROP POLICY IF EXISTS "deny insert for normal users" ON public.jobs;
CREATE POLICY "deny insert for normal users"
  ON public.jobs
  FOR INSERT
  WITH CHECK (false);

DROP POLICY IF EXISTS "deny update for normal users" ON public.jobs;
CREATE POLICY "deny update for normal users"
  ON public.jobs
  FOR UPDATE
  USING (false)
  WITH CHECK (false);

DROP POLICY IF EXISTS "deny delete for normal users" ON public.jobs;
CREATE POLICY "deny delete for normal users"
  ON public.jobs
  FOR DELETE
  USING (false);
