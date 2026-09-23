use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct OrderRequest {
    pub items: Vec<OrderItemRequest>,
}

#[derive(Deserialize)]
pub struct OrderItemRequest {
    pub product_id: i64,
    pub size: String,
    pub quantity: i32,
}

#[derive(Serialize)]
pub struct OrderResponse {
    pub id: String,
    pub total: Decimal,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub items: Vec<OrderItemResponse>,
}

#[derive(Serialize)]
pub struct OrderItemResponse {
    pub product_id: i64,
    pub name: String,
    pub size: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub line_total: Decimal,
}

pub fn size_multiplier(size: &str) -> Option<Decimal> {
    match size {
        "small" => Some(Decimal::new(1, 0)),
        "medium" => Some(Decimal::new(145, 2)),
        "large" => Some(Decimal::new(195, 2)),
        _ => None,
    }
}

pub fn round_to_nearest_100(amount: Decimal) -> Decimal {
    let hundred = Decimal::new(100, 0);
    (amount / hundred).round() * hundred
}