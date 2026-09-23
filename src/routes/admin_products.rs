use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{post, put},
    Extension, Json, Router,
};
use rust_decimal::Decimal;

use crate::auth::AdminId;
use crate::models::product::{Product, ProductInput};
use crate::state::AppState;

// --- Error handling ---
enum ProductError {
    NotFound(i64),
    Invalid(String),
    InUse(i64),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for ProductError {
    fn from(e: sqlx::Error) -> Self {
        ProductError::Db(e)
    }
}

impl IntoResponse for ProductError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ProductError::NotFound(id) => (StatusCode::NOT_FOUND, format!("product {id} not found")),
            ProductError::Invalid(msg) => (StatusCode::BAD_REQUEST, msg),
            ProductError::InUse(id) => (
                StatusCode::CONFLICT,
                format!("product {id} appears in existing orders; hide it instead of deleting"),
            ),
            ProductError::Db(e) => {
                eprintln!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// The DB has CHECK constraints too, but checking here gives the client
// a readable message instead of a generic 500.
fn validate(input: &ProductInput) -> Result<(), ProductError> {
    if input.name.trim().is_empty() {
        return Err(ProductError::Invalid("name is required".into()));
    }
    if input.category.trim().is_empty() {
        return Err(ProductError::Invalid("category is required".into()));
    }
    if input.price < Decimal::ZERO {
        return Err(ProductError::Invalid("price must be >= 0".into()));
    }
    if input.stock < 0 {
        return Err(ProductError::Invalid("stock must be >= 0".into()));
    }
    Ok(())
}

// POST /api/admin/products
async fn create_product(
    State(state): State<AppState>,
    Extension(admin): Extension<AdminId>, // set by require_admin
    Json(input): Json<ProductInput>,
) -> Result<(StatusCode, Json<Product>), ProductError> {
    validate(&input)?;

    let product = sqlx::query_as!(
        Product,
        r#"
        INSERT INTO products (name, description, price, category, image_url, stock, hidden)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, name, description, price, category, image_url, stock, created_at
        "#,
        input.name.trim(),
        input.description,
        input.price,
        input.category.trim(),
        input.image_url,
        input.stock,
        input.hidden
    )
    .fetch_one(&state.pool)
    .await?;

    println!("admin {} created product {}", admin.0, product.id);
    Ok((StatusCode::CREATED, Json(product)))
}

// PUT /api/admin/products/{id} — replaces all editable fields
async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<ProductInput>,
) -> Result<Json<Product>, ProductError> {
    validate(&input)?;

    let product = sqlx::query_as!(
        Product,
        r#"
        UPDATE products
        SET name = $2, description = $3, price = $4, category = $5,
            image_url = $6, stock = $7, hidden = $8
        WHERE id = $1
        RETURNING id, name, description, price, category, image_url, stock, created_at
        "#,
        id,
        input.name.trim(),
        input.description,
        input.price,
        input.category.trim(),
        input.image_url,
        input.stock,
        input.hidden
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ProductError::NotFound(id))?;

    Ok(Json(product))
}

// DELETE /api/admin/products/{id}
async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ProductError> {
    let result = sqlx::query!("DELETE FROM products WHERE id = $1", id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() == 0 => Err(ProductError::NotFound(id)),
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        // order_items.product_id references products(id), so Postgres
        // refuses to delete a product that was ever ordered.
        Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
            Err(ProductError::InUse(id))
        }
        Err(e) => Err(e.into()),
    }
}

// No auth check in here — the router in routes/mod.rs wraps this
// with the require_admin middleware.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/products", post(create_product))
        .route("/api/admin/products/{id}", put(update_product).delete(delete_product))
}
