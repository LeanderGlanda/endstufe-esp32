use axum::{Router, routing::get, extract::Query, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Deserialize, Serialize)]
struct QueryParams {
    name: String,
}

async fn greet(query: Query<QueryParams>) -> impl IntoResponse {
    format!("Hello, {}!", query.name)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(greet));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}