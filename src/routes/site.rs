use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::models::feedback::PublicReview;
use crate::models::product::Product;
use crate::state::AppState;

#[derive(Serialize)]
struct SiteResponse {
    products: Vec<Product>,
    settings: serde_json::Value,
    reviews: Vec<PublicReview>,
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

    // Only the six fields API.md allows, selected by name: phone is never
    // even read from the database here, so it can't leak by accident.
    // `AND consent` repeats the rule as a second safety net.
    let reviews = sqlx::query_as!(
        PublicReview,
        r#"
        SELECT name, area, rating, message, created_at, NULL::text AS "photo"
        FROM feedback
        WHERE status = 'published' AND consent
        ORDER BY created_at DESC
        LIMIT 50
        "#
    )
    .fetch_all(&state.pool)
    .await
    .expect("failed to fetch reviews");

    Json(SiteResponse {
        products,
        settings: serde_json::json!({}),
        reviews,
    })
}

pub fn router(state: AppState) -> Router {
    Router::new().route("/api/site", get(get_site)).with_state(state)
}