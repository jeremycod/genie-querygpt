use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use querygpt_core::agents::intent;
use querygpt_core::schema::registry::SchemaRegistry;

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    user_prompt: String,
}

#[derive(Debug, Serialize)]
struct GenerateResponse {
    workspace: String,
    sql: String,
    explanation: String,
}

async fn generate(Json(req): Json<GenerateRequest>) -> Json<GenerateResponse> {
    let intent = intent::classify(&req.user_prompt);
    // In production: load workspace registry based on intent.workspace
    let _reg = SchemaRegistry::load("config/workspaces/campaigns_offers.index.json").ok();

    Json(GenerateResponse {
        workspace: intent.workspace,
        sql: "-- SQL generation pipeline TBD".to_string(),
        explanation: "-- Explanation TBD".to_string(),
    })
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt().init();
    
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("{}:{}", host, port);
    
    let app = Router::new().route("/generate", post(generate));
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    println!("Server running on {}", bind_addr);
    axum::serve(listener, app).await.unwrap();
}
