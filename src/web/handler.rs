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

pub async fn start_server() {
    log::info!("Starting webserver...");
    let app = Router::new().route("/", get(greet));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/*
let app = Router::new().route("/command", post(handler::handle_command));

    let addr = "0.0.0.0:8080".parse()?;
    println!("Starting server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(()) 
    
    use axum::{Json, response::IntoResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Command {
    action: String,
}

pub async fn handle_command(Json(payload): Json<Command>) -> impl IntoResponse {
    match payload.action.as_str() {
        "mute" => {
            println!("Mute command received");
            // Add hardware interaction logic here
        }
        "volume_up" => {
            println!("Volume up command received");
        }
        "volume_down" => {
            println!("Volume down command received");
        }
        _ => {
            println!("Unknown command: {}", payload.action);
            return "Unknown command".into_response();
        }
    }

    "Command executed".into_response()
}

    
    */