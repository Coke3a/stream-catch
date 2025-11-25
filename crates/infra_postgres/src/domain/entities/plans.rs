use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use crate::infrastructure::postgres::schema::plans;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = plans)]
pub struct PlanEntity {
    pub id: Uuid,
    pub name: Option<String>,
    pub price_minor: i32,
    pub duration_days: i32,
    pub features: Value,
    pub is_active: bool,
}
