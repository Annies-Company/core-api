use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub price: Decimal,      // NUMERIC in Postgres — never use f64 for money
    pub category: String,
    pub image_url: Option<String>,
    pub stock: i32,
    pub created_at: DateTime<Utc>,
}

// Body for creating/replacing a product from the admin panel.
// No id or created_at — the database assigns those.
#[derive(Debug, Deserialize)]
pub struct ProductInput {
    pub name: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub category: String,
    pub image_url: Option<String>,
    #[serde(default)]
    pub stock: i32,
    #[serde(default)]
    pub hidden: bool,
}
