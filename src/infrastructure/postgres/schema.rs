// @generated automatically by Diesel CLI.

diesel::table! {
    app_users (id) {
        id -> Uuid,
        display_name -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    deliveries (id) {
        id -> Int8,
        recording_id -> Int8,
        user_id -> Uuid,
        via -> Text,
        delivered_at -> Nullable<Timestamptz>,
        status -> Text,
        error -> Nullable<Text>,
    }
}

diesel::table! {
    follows (user_id, live_account_id) {
        user_id -> Uuid,
        live_account_id -> Int8,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    invoices (id) {
        id -> Int8,
        user_id -> Uuid,
        subscription_id -> Nullable<Int8>,
        plan_id -> Int8,
        amount_minor -> Int4,
        period_start -> Timestamptz,
        period_end -> Timestamptz,
        due_at -> Timestamptz,
        status -> Text,
        created_at -> Timestamptz,
        paid_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    jobs (id) {
        id -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        payload -> Jsonb,
        run_at -> Timestamptz,
        attempts -> Int4,
        locked_at -> Nullable<Timestamptz>,
        locked_by -> Nullable<Text>,
        error -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    live_accounts (id) {
        id -> Int8,
        platform -> Text,
        account_id -> Text,
        canonical_url -> Text,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    payment_methods (id) {
        id -> Int8,
        user_id -> Uuid,
        provider -> Text,
        method_type -> Text,
        pm_ref -> Text,
        brand -> Nullable<Text>,
        last4 -> Nullable<Text>,
        exp_month -> Nullable<Int4>,
        exp_year -> Nullable<Int4>,
        status -> Text,
        is_default -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    payment_provider_customers (id) {
        id -> Int8,
        user_id -> Uuid,
        provider -> Text,
        customer_ref -> Text,
        metadata -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    payments (id) {
        id -> Int8,
        invoice_id -> Int8,
        user_id -> Uuid,
        provider -> Text,
        method_type -> Text,
        payment_method_id -> Nullable<Int8>,
        amount_minor -> Int4,
        status -> Text,
        provider_payment_id -> Nullable<Text>,
        provider_session_ref -> Nullable<Text>,
        error -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    plans (id) {
        id -> Int8,
        name -> Nullable<Text>,
        price_minor -> Int4,
        duration_days -> Int4,
        features -> Jsonb,
        is_active -> Bool,
    }
}

diesel::table! {
    recordings (id) {
        id -> Int8,
        live_account_id -> Int8,
        recording_key -> Nullable<Text>,
        started_at -> Timestamptz,
        ended_at -> Nullable<Timestamptz>,
        duration_sec -> Nullable<Int4>,
        size_bytes -> Nullable<Int8>,
        storage_prefix -> Nullable<Text>,
        status -> Text,
        poster_storage_path -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    subscriptions (id) {
        id -> Int8,
        user_id -> Uuid,
        plan_id -> Int8,
        starts_at -> Timestamptz,
        ends_at -> Timestamptz,
        billing_mode -> Text,
        default_payment_method_id -> Nullable<Int8>,
        cancel_at_period_end -> Bool,
        canceled_at -> Nullable<Timestamptz>,
        status -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(deliveries -> recordings (recording_id));
diesel::joinable!(deliveries -> app_users (user_id));
diesel::joinable!(follows -> live_accounts (live_account_id));
diesel::joinable!(follows -> app_users (user_id));
diesel::joinable!(invoices -> plans (plan_id));
diesel::joinable!(invoices -> subscriptions (subscription_id));
diesel::joinable!(invoices -> app_users (user_id));
diesel::joinable!(payment_methods -> app_users (user_id));
diesel::joinable!(payment_provider_customers -> app_users (user_id));
diesel::joinable!(payments -> invoices (invoice_id));
diesel::joinable!(payments -> payment_methods (payment_method_id));
diesel::joinable!(payments -> app_users (user_id));
diesel::joinable!(recordings -> live_accounts (live_account_id));
diesel::joinable!(subscriptions -> payment_methods (default_payment_method_id));
diesel::joinable!(subscriptions -> plans (plan_id));
diesel::joinable!(subscriptions -> app_users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    app_users,
    deliveries,
    follows,
    invoices,
    jobs,
    live_accounts,
    payment_methods,
    payment_provider_customers,
    payments,
    plans,
    recordings,
    subscriptions,
);
