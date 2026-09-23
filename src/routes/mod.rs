use axum::Router;

pub mod health;
pub mod ping;

pub fn router() -> axum::Router {
    Router::new()
        .merge(ping::router()).merge(health::router())
}