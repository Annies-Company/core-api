use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;
use tower_sessions::Session;

// Key under which the logged-in admin's id is stored in the session.
pub const ADMIN_ID_KEY: &str = "admin_id";

// Put into request extensions by `require_admin`, so handlers behind it
// can take `Extension(admin): Extension<AdminId>` to know who's calling.
#[derive(Clone, Copy)]
pub struct AdminId(pub i64);

// Middleware: runs before every handler it wraps. Like a Django
// @login_required decorator, but applied to a whole group of routes.
//
// `session` is pulled out by the same extractor system handlers use;
// `next.run(req)` calls the actual handler. Returning early without
// calling it means the handler never runs.
pub async fn require_admin(session: Session, mut req: Request, next: Next) -> Response {
    let admin_id = match session.get::<i64>(ADMIN_ID_KEY).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "not logged in" })),
            )
                .into_response();
        }
        Err(e) => {
            eprintln!("session error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    req.extensions_mut().insert(AdminId(admin_id));
    next.run(req).await
}

// Like Django's make_password: returns a PHC string ($argon2id$v=19$...)
// that contains the algorithm, params, random salt and hash together.
pub fn hash_password(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .expect("failed to hash password")
        .to_string()
}

// Like Django's check_password. Params are read from the stored hash,
// so old hashes keep working even if defaults change later.
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

// `cargo run -- --create-admin you@example.com`
// Our version of `manage.py createsuperuser`. There is no public signup
// route, so this is the only way an admin account gets created.
pub async fn create_admin(pool: &PgPool, email: &str) {
    let email = email.trim().to_lowercase();

    // Prompt instead of taking the password as an argument, so it
    // doesn't end up in shell history. Input is hidden while typing.
    let password = rpassword::prompt_password("Password: ").expect("failed to read password");
    let confirm = rpassword::prompt_password("Confirm password: ").expect("failed to read password");

    if password != confirm {
        eprintln!("passwords do not match");
        std::process::exit(1);
    }
    if password.len() < 8 {
        eprintln!("password must be at least 8 characters");
        std::process::exit(1);
    }

    let hash = hash_password(&password);

    let result = sqlx::query!(
        r#"INSERT INTO admins (email, password_hash) VALUES ($1, $2) RETURNING id"#,
        email,
        hash
    )
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => println!("created admin {email} (id {})", row.id),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            eprintln!("an admin with email {email} already exists");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("failed to create admin: {e}");
            std::process::exit(1);
        }
    }
}
