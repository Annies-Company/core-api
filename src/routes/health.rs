use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]

pub struct Health {
    status: &'static str
}

async fn health() -> Json<Health> {
    Json(Health{
            status: "ok"
    }
)

}
        
    


pub fn router() -> Router {
    Router::new().route("/health", get(health))
}