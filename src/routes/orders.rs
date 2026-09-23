use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use rand::RngExt;
use rust_decimal::Decimal;

use crate::models::order::{
    round_to_nearest_100, size_multiplier, OrderItemRequest, OrderItemResponse, OrderRequest,
    OrderResponse,
};
use crate::state::AppState;

// --- Error handling ---
// A public write endpoint taking untrusted input needs real error
// responses, not just .expect() panics that 500 on any bad input.
enum OrderError {
    ProductNotFound(i64),
    InvalidSize(String),
    EmptyOrder,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for OrderError {
    fn from(e: sqlx::Error) -> Self {
        OrderError::Db(e)
    }
}

impl IntoResponse for OrderError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            OrderError::ProductNotFound(id) => {
                (StatusCode::BAD_REQUEST, format!("product {id} not found"))
            }
            OrderError::InvalidSize(s) => (
                StatusCode::BAD_REQUEST,
                format!("invalid size '{s}', must be small, medium, or large"),
            ),
            OrderError::EmptyOrder => {
                (StatusCode::BAD_REQUEST, "order must have at least one item".to_string())
            }
            OrderError::Db(e) => {
                eprintln!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// --- Order id: AC-XXXXXXXX ---
fn generate_order_id() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    let suffix: String = (0..8)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    format!("AC-{suffix}")
}

async fn create_order(
    State(state): State<AppState>,
    Json(body): Json<OrderRequest>,
) -> Result<Json<OrderResponse>, OrderError> {
    if body.items.is_empty() {
        return Err(OrderError::EmptyOrder);
    }

    let mut tx = state.pool.begin().await?;

    let mut items_out: Vec<OrderItemResponse> = Vec::with_capacity(body.items.len());
    let mut total = Decimal::ZERO;

    for OrderItemRequest {
        product_id,
        size,
        quantity,
    } in body.items
    {
        let multiplier = size_multiplier(&size).ok_or(OrderError::InvalidSize(size.clone()))?;

        // THE important line: fetch the real price from the DB, inside
        // the transaction. Whatever price the client may have sent (if
        // any) is never looked at — it isn't even part of OrderRequest.
        let product = sqlx::query!(
            r#"SELECT name, price FROM products WHERE id = $1 AND hidden = false"#,
            product_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(OrderError::ProductNotFound(product_id))?;

        let unit_price = round_to_nearest_100(product.price * multiplier);
        let line_total = unit_price * Decimal::from(quantity);
        total += line_total;

        items_out.push(OrderItemResponse {
            product_id,
            name: product.name,
            size,
            quantity,
            unit_price,
            line_total,
        });
    }

    let id = generate_order_id();

    let order_row = sqlx::query!(
        r#"INSERT INTO orders (id, total) VALUES ($1, $2) RETURNING status, created_at"#,
        id,
        total
    )
    .fetch_one(&mut *tx)
    .await?;

    for item in &items_out {
        sqlx::query!(
            r#"INSERT INTO order_items (order_id, product_id, size, quantity, unit_price, line_total)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
            id,
            item.product_id,
            item.size,
            item.quantity,
            item.unit_price,
            item.line_total
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(OrderResponse {
        id,
        total,
        status: order_row.status,
        created_at: order_row.created_at,
        items: items_out,
    }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/orders", post(create_order))
        .with_state(state)
}