use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};

use crate::models::feedback::{Feedback, StatusUpdate, STATUSES};
use crate::state::AppState;

// --- Error handling ---
enum AdminFeedbackError {
    NotFound,
    InvalidStatus,
    NoConsent(String),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AdminFeedbackError {
    fn from(e: sqlx::Error) -> Self {
        AdminFeedbackError::Db(e)
    }
}

impl IntoResponse for AdminFeedbackError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AdminFeedbackError::NotFound => {
                (StatusCode::NOT_FOUND, "That feedback is no longer here.".to_string())
            }
            AdminFeedbackError::InvalidStatus => (
                StatusCode::BAD_REQUEST,
                "status must be one of new, read, published, hidden".to_string(),
            ),
            // Same wording as the preview mode in the React app's api.js.
            AdminFeedbackError::NoConsent(name) => (
                StatusCode::FORBIDDEN,
                format!("{name} did not agree to this being shown on the website."),
            ),
            AdminFeedbackError::Db(e) => {
                eprintln!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// GET /api/admin/feedback — newest first, INCLUDING phone (admin only)
async fn list_feedback(
    State(state): State<AppState>,
) -> Result<Json<Vec<Feedback>>, AdminFeedbackError> {
    let rows = sqlx::query_as!(
        Feedback,
        r#"
        SELECT id, status, created_at, rating, name, area, phone, ordered, message, consent
        FROM feedback
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// PATCH /api/admin/feedback/{id}  body: { "status": "published" }
async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<StatusUpdate>,
) -> Result<Json<Feedback>, AdminFeedbackError> {
    if !STATUSES.contains(&body.status.as_str()) {
        return Err(AdminFeedbackError::InvalidStatus);
    }

    let entry = sqlx::query!("SELECT name, consent FROM feedback WHERE id = $1", id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AdminFeedbackError::NotFound)?;

    // THE rule: no consent → never published. The admin panel hides the
    // button, but a hidden button is not a guarantee — anyone with a
    // session could send this request by hand. So the server says no.
    if body.status == "published" && !entry.consent {
        return Err(AdminFeedbackError::NoConsent(entry.name));
    }

    let result = sqlx::query_as!(
        Feedback,
        r#"
        UPDATE feedback SET status = $2 WHERE id = $1
        RETURNING id, status, created_at, rating, name, area, phone, ordered, message, consent
        "#,
        id,
        body.status
    )
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(updated)) => Ok(Json(updated)),
        Ok(None) => Err(AdminFeedbackError::NotFound),
        // Last line of defence: the feedback_publish_needs_consent CHECK
        // in the migration. Only reachable if the check above is ever
        // removed or bypassed — still answered as a 403, not a 500.
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("feedback_publish_needs_consent") =>
        {
            Err(AdminFeedbackError::NoConsent(entry.name))
        }
        Err(e) => Err(e.into()),
    }
}

// DELETE /api/admin/feedback/{id}
async fn delete_feedback(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AdminFeedbackError> {
    let result = sqlx::query!("DELETE FROM feedback WHERE id = $1", id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminFeedbackError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// Wrapped with require_admin in routes/mod.rs.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/feedback", get(list_feedback))
        .route(
            "/api/admin/feedback/{id}",
            patch(update_status).delete(delete_feedback),
        )
}
