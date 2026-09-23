use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tower_sessions::Session;

use crate::auth::{verify_password, ADMIN_ID_KEY};
use crate::models::admin::{AdminResponse, AdminRow, LoginRequest};
use crate::state::AppState;

// --- Error handling ---
enum AuthError {
    InvalidCredentials,
    NotLoggedIn,
    Db(sqlx::Error),
    Session(tower_sessions::session::Error),
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        AuthError::Db(e)
    }
}

impl From<tower_sessions::session::Error> for AuthError {
    fn from(e: tower_sessions::session::Error) -> Self {
        AuthError::Session(e)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            // Same message for "no such email" and "wrong password",
            // so attackers can't probe which emails have accounts.
            AuthError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "invalid email or password".to_string())
            }
            AuthError::NotLoggedIn => (StatusCode::UNAUTHORIZED, "not logged in".to_string()),
            AuthError::Db(e) => {
                eprintln!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
            AuthError::Session(e) => {
                eprintln!("session error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// POST /api/admin/login
async fn login(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AdminResponse>, AuthError> {
    let email = body.email.trim().to_lowercase();

    let admin = sqlx::query_as!(
        AdminRow,
        r#"SELECT id, email, password_hash, created_at FROM admins WHERE email = $1"#,
        email
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AuthError::InvalidCredentials)?;

    if !verify_password(&body.password, &admin.password_hash) {
        return Err(AuthError::InvalidCredentials);
    }

    // New session id on login, so a session id planted before login
    // (session fixation) is useless afterwards. Django does the same.
    session.cycle_id().await?;
    session.insert(ADMIN_ID_KEY, admin.id).await?;

    Ok(Json(admin.into()))
}

// GET /api/admin/me — our `request.user`
async fn me(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<AdminResponse>, AuthError> {
    let admin_id: i64 = session
        .get(ADMIN_ID_KEY)
        .await?
        .ok_or(AuthError::NotLoggedIn)?;

    let admin = sqlx::query_as!(
        AdminRow,
        r#"SELECT id, email, password_hash, created_at FROM admins WHERE id = $1"#,
        admin_id
    )
    .fetch_optional(&state.pool)
    .await?
    // Admin was deleted while still logged in.
    .ok_or(AuthError::NotLoggedIn)?;

    Ok(Json(admin.into()))
}

// POST /api/admin/logout
async fn logout(session: Session) -> Result<StatusCode, AuthError> {
    // Deletes the session data server-side and expires the cookie.
    session.flush().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/admin/login", post(login))
        .route("/api/admin/me", get(me))
        .route("/api/admin/logout", post(logout))
        .with_state(state)
}
