use axum::{routing::get, Router};


async fn ping() -> &'static str {
    "pong"
}

pub fn router() -> Router {
    Router::new().route("/ping", get(ping))
}