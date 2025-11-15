-- Extensions
CREATE EXTENSION IF NOT EXISTS citext;

-- ================
-- Core tables
-- ================

CREATE TABLE users (
  id BIGSERIAL PRIMARY KEY,
  email citext UNIQUE,
  username TEXT,
  password_hash TEXT,
  telegram_id BIGINT,
  status TEXT CHECK (status IN ('active','blocked')) DEFAULT 'active',
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now()
);

CREATE TABLE password_reset_tokens (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  token_hash TEXT UNIQUE NOT NULL,
  expires_at timestamptz NOT NULL,
  used_at timestamptz,
  created_at timestamptz DEFAULT now()
);

CREATE TABLE plans (
  id BIGSERIAL PRIMARY KEY,
  name TEXT,
  price_minor INT NOT NULL,
  duration_days INT NOT NULL,
  features JSONB NOT NULL DEFAULT '{}'::jsonb,
  is_active BOOLEAN DEFAULT true
);

CREATE TABLE payment_provider_customers (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  provider TEXT NOT NULL,
  customer_ref TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at timestamptz DEFAULT now()
);

CREATE TABLE payment_methods (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  provider TEXT NOT NULL,
  method_type TEXT NOT NULL CHECK (method_type IN ('card','promptpay')),
  pm_ref TEXT NOT NULL,
  brand TEXT,
  last4 TEXT,
  exp_month INT,
  exp_year INT,
  status TEXT NOT NULL CHECK (status IN ('active','inactive','expired')) DEFAULT 'active',
  is_default BOOLEAN DEFAULT false,
  created_at timestamptz DEFAULT now()
);

CREATE TABLE subscriptions (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  plan_id BIGINT NOT NULL,
  starts_at timestamptz NOT NULL,
  ends_at timestamptz NOT NULL,
  billing_mode TEXT NOT NULL CHECK (billing_mode IN ('recurring','manual')) DEFAULT 'recurring',
  default_payment_method_id BIGINT,
  cancel_at_period_end BOOLEAN DEFAULT false,
  canceled_at timestamptz,
  status TEXT NOT NULL CHECK (status IN ('active','past_due','canceled','expired')),
  created_at timestamptz DEFAULT now()
);

CREATE TABLE invoices (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  subscription_id BIGINT,
  plan_id BIGINT NOT NULL,
  amount_minor INT NOT NULL CHECK (amount_minor >= 0),
  period_start timestamptz NOT NULL,
  period_end timestamptz NOT NULL,
  due_at timestamptz NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending','paid','failed','void','past_due')),
  created_at timestamptz DEFAULT now(),
  paid_at timestamptz
);

CREATE TABLE payments (
  id BIGSERIAL PRIMARY KEY,
  invoice_id BIGINT NOT NULL,
  user_id BIGINT NOT NULL,
  provider TEXT NOT NULL,
  method_type TEXT NOT NULL CHECK (method_type IN ('card','promptpay')),
  payment_method_id BIGINT,
  amount_minor INT NOT NULL CHECK (amount_minor >= 0),
  status TEXT NOT NULL CHECK (status IN ('requires_action','processing','succeeded','failed','canceled')),
  provider_payment_id TEXT,
  provider_session_ref TEXT,
  error TEXT,
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now()
);

CREATE TABLE live_accounts (
  id BIGSERIAL PRIMARY KEY,
  platform TEXT NOT NULL,
  account_id TEXT NOT NULL,
  canonical_url TEXT NOT NULL,
  status TEXT CHECK (status IN ('active','paused','error')) DEFAULT 'active',
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now()
);

CREATE TABLE follows (
  user_id BIGINT NOT NULL,
  live_account_id BIGINT NOT NULL,
  status TEXT CHECK (status IN ('active','inactive','temporary_inactive')) DEFAULT 'active',
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now(),
  PRIMARY KEY (user_id, live_account_id)
);

CREATE TABLE recordings (
  id BIGSERIAL PRIMARY KEY,
  live_account_id BIGINT NOT NULL,
  recording_key TEXT UNIQUE,
  started_at timestamptz NOT NULL,
  ended_at timestamptz NOT NULL,
  duration_sec INT,
  size_bytes BIGINT,
  storage_prefix TEXT,
  status TEXT CHECK (status IN ('processing','ready','failed')),
  poster_key TEXT,
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now()
);

CREATE TABLE deliveries (
  id BIGSERIAL PRIMARY KEY,
  recording_id BIGINT NOT NULL,
  user_id BIGINT NOT NULL,
  via TEXT NOT NULL CHECK (via IN ('web_notify','email','telegram')),
  delivered_at timestamptz,
  status TEXT CHECK (status IN ('queued','sent','failed')),
  error TEXT
);

CREATE TABLE jobs (
  id BIGSERIAL PRIMARY KEY,
  type TEXT NOT NULL CHECK (type IN ('RecordingUpload','NotifyReady')),
  payload JSONB NOT NULL,
  run_at timestamptz,
  attempts INT NOT NULL DEFAULT 0,
  locked_at timestamptz,
  locked_by TEXT,
  status TEXT NOT NULL CHECK (status IN ('queued','running','done','failed','dead')) DEFAULT 'queued'
  error TEXT,
  created_at timestamptz NOT NULL DEFAULT (now())
);

-- ================
-- Indexes
-- ================

-- Payment methods (fix inline index -> standalone)
CREATE INDEX pm_user_idx ON payment_methods (user_id);

CREATE UNIQUE INDEX ON subscriptions (user_id, plan_id, starts_at);
CREATE UNIQUE INDEX ON payment_provider_customers (user_id, provider);
CREATE UNIQUE INDEX ON payment_provider_customers (provider, customer_ref);
CREATE UNIQUE INDEX ON payment_methods (provider, pm_ref);
CREATE UNIQUE INDEX ON invoices (subscription_id, period_start);
CREATE INDEX invoices_user_status_idx ON invoices (user_id, status);
CREATE INDEX invoices_due_idx ON invoices (status, due_at);
CREATE INDEX payments_invoice_idx ON payments (invoice_id);
CREATE UNIQUE INDEX ON live_accounts (platform, account_id);
CREATE UNIQUE INDEX ON deliveries (recording_id, user_id, via);
CREATE INDEX jobs_status_run_at_idx ON jobs (status, run_at);

-- ================
-- Foreign keys
-- ================

ALTER TABLE password_reset_tokens
  ADD FOREIGN KEY (user_id) REFERENCES users (id);

ALTER TABLE subscriptions
  ADD FOREIGN KEY (user_id) REFERENCES users (id),
  ADD FOREIGN KEY (plan_id) REFERENCES plans (id),
  ADD FOREIGN KEY (default_payment_method_id) REFERENCES payment_methods (id);

ALTER TABLE payment_provider_customers
  ADD FOREIGN KEY (user_id) REFERENCES users (id);

ALTER TABLE payment_methods
  ADD FOREIGN KEY (user_id) REFERENCES users (id);

ALTER TABLE invoices
  ADD FOREIGN KEY (user_id) REFERENCES users (id),
  ADD FOREIGN KEY (subscription_id) REFERENCES subscriptions (id),
  ADD FOREIGN KEY (plan_id) REFERENCES plans (id);

ALTER TABLE payments
  ADD FOREIGN KEY (invoice_id) REFERENCES invoices (id),
  ADD FOREIGN KEY (user_id) REFERENCES users (id),
  ADD FOREIGN KEY (payment_method_id) REFERENCES payment_methods (id);

ALTER TABLE follows
  ADD FOREIGN KEY (user_id) REFERENCES users (id),
  ADD FOREIGN KEY (live_account_id) REFERENCES live_accounts (id);

ALTER TABLE recordings
  ADD FOREIGN KEY (live_account_id) REFERENCES live_accounts (id);

ALTER TABLE deliveries
  ADD FOREIGN KEY (recording_id) REFERENCES recordings (id),
  ADD FOREIGN KEY (user_id) REFERENCES users (id);
