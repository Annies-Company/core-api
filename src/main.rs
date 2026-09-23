
use axum::{Router, routing::get, routing::post, Json, extract::Path};

use serde::{Deserialize, Serialize};


mod routes;


async fn greet(Path(name): Path<String>) -> String {
    format!("Hello, {name}!")
}
#[derive(Deserialize)]

pub struct EchoIn {
    name: String,
}
#[derive(Serialize)]

pub struct EchoOut {
    name: String,
    length: usize
}
async fn echo(Json(message): Json<EchoIn>) -> Json<EchoOut> {
    Json(EchoOut {
        length: message.name.len(),
        name: message.name
    })
}
#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // let app = Router::new()
    //     .route("/", get(greet))
    //     .route("/echo", get(echo));
    let app = Router::new()
            .merge(routes::router())
            .route("/greet/{name}", get(greet))
            .route("/echo", post(echo));



    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("ailed to bind to 8080 ");

    println!("Listening on http://0.0.0.0:8080");

    axum::serve(listener, app).await.expect("error");
}


