use diesel::prelude::*;
use crate::infrastructure::postgres::schema::plans;

#[derive(Debug, Clone, Identifiable, Selectable, Queryable)]
#[diesel(table_name = plans)]
pub struct PlanEntity {
    pub id: i32,
    pub name: i32,
    pub price_minor: i32,
    pub duration_days: i32,
    pub features: String,
    pub is_active: bool,
}