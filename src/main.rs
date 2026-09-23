
// use axum::{Router, routing::get, routing::post, Json, extract::Path};

// use serde::{Deserialize, Serialize};


// mod routes;


// async fn greet(Path(name): Path<String>) -> String {
//     format!("Hello, {name}!")
// }
// #[derive(Deserialize)]

// pub struct EchoIn {
//     name: String,
// }
// #[derive(Serialize)]

// pub struct EchoOut {
//     name: String,
//     length: usize
// }
// async fn echo(Json(message): Json<EchoIn>) -> Json<EchoOut> {
//     Json(EchoOut {
//         length: message.name.len(),
//         name: message.name
//     })
// }
// #[tokio::main]
// async fn main() {
//     println!("Hello, world!");

//     // let app = Router::new()
//     //     .route("/", get(greet))
//     //     .route("/echo", get(echo));
//     let app = Router::new()
//             .merge(routes::router())
//             .route("/greet/{name}", get(greet))
//             .route("/echo", post(echo));



//     let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
//         .await
//         .expect("ailed to bind to 8080 ");

//     println!("Listening on http://0.0.0.0:8080");

//     axum::serve(listener, app).await.expect("error");
// }


use axum::{
    extract::{Json, Path},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_sessions::{cookie::SameSite, MemoryStore, SessionManagerLayer};

mod auth;
mod ids;
mod models;
mod rate_limit;
mod routes;
mod state;
use state::AppState;

async fn greet(Path(name): Path<String>) -> String {
    format!("hello, {name}")
}

#[derive(Deserialize)]
struct EchoIn {
    message: String,
}

#[derive(Serialize)]
struct EchoOut {
    you_said: String,
    length: usize,
}

async fn echo(Json(body): Json<EchoIn>) -> Json<EchoOut> {
    Json(EchoOut {
        length: body.message.len(),
        you_said: body.message,
    })
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // --- DB checkpoint: connect, run migrations, raw SELECT 1 ---
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("raw SELECT 1 failed");
    println!("DB checkpoint OK, SELECT 1 returned: {result}");

    // --- end checkpoint ---

    // `cargo run -- --create-admin you@example.com` creates an admin and
    // exits without starting the server (like `manage.py createsuperuser`).
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--create-admin") {
        let Some(email) = args.get(2) else {
            eprintln!("usage: cargo run -- --create-admin <email>");
            std::process::exit(1);
        };
        auth::create_admin(&pool, email).await;
        return;
    }

    let storage = state::Storage::from_env();
    if storage.is_none() {
        println!("S3_* env vars not set: image uploads disabled");
    }
    let state = AppState { pool, storage };

    // Sessions live in memory for now: everyone is logged out on restart.
    // The cookie only holds a random session id; the data stays server-side.
    // COOKIE_SECURE=true in production: the cookie is then only sent over
    // HTTPS. Unset locally, so it still works over plain http://localhost.
    let cookie_secure = std::env::var("COOKIE_SECURE").is_ok_and(|v| v == "true");
    let session_layer =
        SessionManagerLayer::new(MemoryStore::default())
            .with_secure(cookie_secure)
            .with_same_site(SameSite::Lax); // what API.md asks for; HttpOnly is on by default

    // CORS: which websites' JavaScript may call this API *with cookies*.
    // Must be exact origins (never "*" when credentials are allowed).
    // FRONTEND_ORIGINS is comma-separated; defaults to the Vite dev server.
    let origins: Vec<HeaderValue> = std::env::var("FRONTEND_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173".to_string())
        .split(',')
        .map(|o| o.trim().parse().expect("invalid origin in FRONTEND_ORIGINS"))
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .merge(routes::router(state))
        .route("/greet/{name}", axum::routing::get(greet)) // axum 0.8 syntax
        .route("/echo", post(echo))
        .layer(session_layer)
        // Added last = outermost: answers browser preflight (OPTIONS)
        // requests before anything else runs, and adds CORS headers to
        // every response, including 401s and 429s.
        .layer(cors);

    // Hosts like Render/Railway choose the port and pass it in via PORT.
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));

    println!("listening on http://{addr}");
    // with_connect_info exposes the caller's socket address, which the
    // rate limiter uses to tell clients apart.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("server error");
}