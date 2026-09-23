use axum::{middleware, Router};

use crate::auth::require_admin;
use crate::rate_limit::public_write_limit;
use crate::state::AppState;

pub mod admin;
pub mod admin_feedback;
pub mod admin_products;
pub mod feedback;
pub mod health;
pub mod ping;
pub mod site;
pub mod orders;
pub mod uploads;

pub fn router(state: AppState) -> axum::Router {
    // Every route in here requires a logged-in admin. route_layer (not
    // layer) runs the middleware only when a route actually matched, so
    // an unknown URL still gets a 404 instead of a misleading 401.
    let protected = Router::new()
        .merge(admin_products::router())
        .merge(admin_feedback::router())
        .merge(uploads::router())
        .route_layer(middleware::from_fn(require_admin))
        .with_state(state.clone());

    Router::new()
        .merge(ping::router())
        .merge(health::router())
        .merge(site::router(state.clone()))
        // The two routes anyone on the internet can write to: each gets
        // its own per-IP rate limit.
        .merge(orders::router(state.clone()).route_layer(public_write_limit()))
        .merge(feedback::router(state.clone()).route_layer(public_write_limit()))
        .merge(admin::router(state))
        .merge(protected)
}
