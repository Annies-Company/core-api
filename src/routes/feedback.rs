use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

use crate::models::feedback::{FeedbackInput, FeedbackReceipt};
use crate::state::AppState;

// --- Error handling ---
enum FeedbackError {
    Invalid(String),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for FeedbackError {
    fn from(e: sqlx::Error) -> Self {
        FeedbackError::Db(e)
    }
}

impl IntoResponse for FeedbackError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            FeedbackError::Invalid(msg) => (StatusCode::BAD_REQUEST, msg),
            FeedbackError::Db(e) => {
                eprintln!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// Trims, and enforces a max length in characters (not bytes, so names
// in any script are measured fairly). Missing becomes "".
fn clean(value: Option<String>, label: &str, max: usize) -> Result<String, FeedbackError> {
    let value = value.unwrap_or_default().trim().to_string();
    if value.chars().count() > max {
        return Err(FeedbackError::Invalid(format!(
            "{label} is too long (max {max} characters)"
        )));
    }
    Ok(value)
}

fn required(value: String, label: &str, max: usize) -> Result<String, FeedbackError> {
    let value = clean(Some(value), label, max)?;
    if value.is_empty() {
        return Err(FeedbackError::Invalid(format!("{label} is required")));
    }
    Ok(value)
}

// POST /api/feedback — public, rate-limited (see routes/mod.rs)
async fn submit_feedback(
    State(state): State<AppState>,
    Json(input): Json<FeedbackInput>,
) -> Result<(StatusCode, Json<FeedbackReceipt>), FeedbackError> {
    if !(1..=5).contains(&input.rating) {
        return Err(FeedbackError::Invalid("rating must be between 1 and 5".into()));
    }
    let name = required(input.name, "Name", 80)?;
    let message = required(input.message, "Feedback", 3000)?;
    let area = clean(input.area, "Area", 80)?;
    let phone = clean(input.phone, "Phone", 30)?;
    let ordered = clean(input.ordered, "What you ordered", 200)?;

    let id = crate::ids::short_id("FB");

    // RETURNING deliberately leaves out phone: this response goes back to
    // an anonymous caller, and phone is never returned publicly.
    let receipt = sqlx::query_as!(
        FeedbackReceipt,
        r#"
        INSERT INTO feedback (id, rating, name, area, phone, ordered, message, consent)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, status, created_at, rating, name, area, ordered, message, consent
        "#,
        id,
        input.rating,
        name,
        area,
        phone,
        ordered,
        message,
        input.consent
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(receipt)))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/feedback", post(submit_feedback))
        .with_state(state)
}
