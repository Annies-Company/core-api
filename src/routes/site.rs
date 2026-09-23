use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::models::product::Product;
use crate::state::AppState;

#[derive(Serialize)]
struct SiteResponse {
    products: Vec<Product>,
    settings: serde_json::Value,
    reviews: Vec<serde_json::Value>,
}

async fn get_site(State(state): State<AppState>) -> Json<SiteResponse> {
    let products = sqlx::query_as!(
        Product,
        r#"
        SELECT id, name, description, price, category, image_url, stock, created_at
        FROM products
        WHERE hidden = false
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&state.pool)
    .await
    .expect("failed to fetch products");

    Json(SiteResponse {
        products,
        settings: serde_json::json!({}),
        reviews: vec![],
    })
}

pub fn router(state: AppState) -> Router {
    Router::new().route("/api/site", get(get_site)).with_state(state)
}