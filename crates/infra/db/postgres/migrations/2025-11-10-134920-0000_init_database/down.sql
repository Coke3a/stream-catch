-- This file should undo anything in `up.sql`
-- Drop in reverse dependency order; CASCADE cleans up FKs & indexes

DROP TABLE IF EXISTS jobs CASCADE;
DROP TABLE IF EXISTS deliveries CASCADE;
DROP TABLE IF EXISTS recordings CASCADE;
DROP TABLE IF EXISTS follows CASCADE;
DROP TABLE IF EXISTS payments CASCADE;
DROP TABLE IF EXISTS invoices CASCADE;
DROP TABLE IF EXISTS subscriptions CASCADE;
DROP TABLE IF EXISTS payment_methods CASCADE;
DROP TABLE IF EXISTS payment_provider_customers CASCADE;
DROP TABLE IF EXISTS live_accounts CASCADE;
DROP TABLE IF EXISTS plans CASCADE;
DROP TABLE IF EXISTS password_reset_tokens CASCADE;
DROP TABLE IF EXISTS app_users CASCADE;

-- Finally, remove extension used by users.email
DROP EXTENSION IF EXISTS citext;
DROP EXTENSION IF EXISTS pgcrypto;
