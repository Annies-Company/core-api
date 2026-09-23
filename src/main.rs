
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
use tower_sessions::{MemoryStore, SessionManagerLayer};

mod auth;
mod models;
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
    // with_secure(false) lets the cookie work over plain http://localhost —
    // set it to true in production (HTTPS).
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);

    let app = Router::new()
        .merge(routes::router(state))
        .route("/greet/{name}", axum::routing::get(greet)) // axum 0.8 syntax
        .route("/echo", post(echo))
        .layer(session_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind to port 8080");

    println!("listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await.expect("server error");
}