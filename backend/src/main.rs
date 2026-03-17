use axum::{routing::{get, post}, Router};
use db::Db;
use dotenvy;
use std::{collections::HashMap, env, sync::{Arc, RwLock}};

use crate::routes::user::{create_user, signin};
use crate::routes::market::{
    get_markets, get_market, get_orderbook, get_trades, get_user_orders, get_user_trades,
};
use crate::state::state::AppState;
pub mod routes;
pub mod middleware;
pub mod state;

#[tokio::main]
async fn main() {
    dotenvy::from_filename("backend/.env").ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Arc::new(Db::new(&db_url).await.unwrap());

    let app_state = AppState {
        db,
        markets: Arc::new(RwLock::new(HashMap::new())),
    };
    
    let app = Router::new()
        .route("/signup", post(create_user))
        .route("/signin", post(signin))
        .route("/markets", get(get_markets))
        .route("/markets/{market_id}", get(get_market))
        .route("/markets/{market_id}/orderbook", get(get_orderbook))
        .route("/markets/{market_id}/trades", get(get_trades))
        .route("/markets/{market_id}/orders/{user_pubkey}", get(get_user_orders))
        .route("/user/{user_pubkey}/trades", get(get_user_trades))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    println!("Server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await.unwrap();
}