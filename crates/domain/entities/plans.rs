use diesel::prelude::*;
use uuid::Uuid;

use crate::{domain::value_objects::plans::PlanFeatures, infra::db::postgres::schema::plans};

#[derive(Debug, Clone)]
pub struct PlanEntity {
    pub id: Uuid,
    pub name: Option<String>,
    pub price_minor: i32,
    pub duration_days: i32,
    pub features: PlanFeatures,
    pub is_active: bool,
    pub stripe_price_recurring: Option<String>,
    pub stripe_price_one_time_card: Option<String>,
    pub stripe_price_one_time_promptpay: Option<String>,
}

/// Raw row used for Diesel queries. Features stay as JSON and are parsed into PlanFeatures.
#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = plans)]
pub struct PlanRow {
    pub id: Uuid,
    pub name: Option<String>,
    pub price_minor: i32,
    pub duration_days: i32,
    pub features: serde_json::Value,
    pub is_active: bool,
    pub stripe_price_recurring: Option<String>,
    pub stripe_price_one_time_card: Option<String>,
    pub stripe_price_one_time_promptpay: Option<String>,
}

impl From<PlanRow> for PlanEntity {
    fn from(value: PlanRow) -> Self {
        let features = serde_json::from_value(value.features).unwrap_or_default();

        Self {
            id: value.id,
            name: value.name,
            price_minor: value.price_minor,
            duration_days: value.duration_days,
            features,
            is_active: value.is_active,
            stripe_price_recurring: value.stripe_price_recurring,
            stripe_price_one_time_card: value.stripe_price_one_time_card,
            stripe_price_one_time_promptpay: value.stripe_price_one_time_promptpay,
        }
    }
}
