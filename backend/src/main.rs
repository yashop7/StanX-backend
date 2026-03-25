use axum::{Router, routing::{ get, post}};
use db::Db;
use dotenvy;
use std::{collections::HashMap, env, sync::{Arc, RwLock}};

use crate::{bootstrap::bootstrap, receiver::run, routes::{user::{create_user, signin}, ws::ws_handler}};
use crate::routes::market::{
    get_markets, get_market, get_orderbook, get_trades, get_user_orders, get_user_trades,
};
use crate::state::state::AppState;
pub mod routes;
pub mod middleware;
pub mod state;
pub mod bootstrap;
pub mod receiver;

#[tokio::main]
async fn main() {
    log::info!("Hello ");

    dotenvy::from_filename("backend/.env").ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Arc::new(Db::new(&db_url).await.unwrap());

    let app_state = AppState {
        db,
        orderbook: Arc::new(RwLock::new(HashMap::new())),
        ob_channels: Arc::new(RwLock::new(HashMap::new()))
    };

    let redis_port =
        env::var("REDIS_PORT").map_err(|_| anyhow::anyhow!("REDIS_PORT not set in environment")).unwrap();
    let redis_address = env::var("REDIS_ADDRESS")
        .map_err(|_| anyhow::anyhow!("REDIS_ADDRESS not set in environment")).unwrap();
    let redis_url = format!("redis://{}:{}", redis_address, redis_port);
log::info!("Hello ");
// println!("Hello");


    let _ = bootstrap(&app_state).await;

    let receiver_state = app_state.clone();
    tokio::spawn(async move {
        run(&receiver_state, redis_url).await;
    });
    
    let app = Router::new()
        .route("/ws/:market_id", get(ws_handler))
        .route("/signup", post(create_user))
        .route("/signin", post(signin))
        .route("/markets", get(get_markets))
        .route("/markets/:market_id", get(get_market))
        .route("/markets/:market_id/orderbook", get(get_orderbook))
        .route("/markets/:market_id/trades", get(get_trades)) // trades?limit=50
        .route("/markets/:market_id/orders/:user_pubkey", get(get_user_orders))
        .route("/user/:user_pubkey/trades", get(get_user_trades))// trades?limit=50
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003")
        .await
        .unwrap();
    
    println!("Server running on http://0.0.0.0:3003");
    
    axum::serve(listener, app).await.unwrap();
}