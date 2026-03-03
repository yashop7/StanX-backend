use axum::{routing::post, Router};
use db::Db;
use dotenvy;
use std::sync::Arc;

use crate::routes::user::{create_user, signin};
use crate::state::state::AppState;
pub mod routes;
pub mod middleware;
pub mod state;

#[tokio::main]
async fn main() {
    dotenvy::from_filename("backend/.env").ok();
    let db = Arc::new(Db::new().await.unwrap());
    
    let app_state = AppState { db };
    
    let app = Router::new()
        .route("/signup", post(create_user))
        .route("/signin", post(signin))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    println!("Server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await.unwrap();
}