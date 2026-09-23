use axum::{middleware, Router};

use crate::auth::require_admin;
use crate::state::AppState;

pub mod admin;
pub mod admin_products;
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
        .merge(uploads::router())
        .route_layer(middleware::from_fn(require_admin))
        .with_state(state.clone());

    Router::new()
        .merge(ping::router())
        .merge(health::router())
        .merge(site::router(state.clone()))
        .merge(orders::router(state.clone()))
        .merge(admin::router(state))
        .merge(protected)
}
