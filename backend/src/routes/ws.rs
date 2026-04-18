use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use axum::extract::ws::*;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use crate::state::state::AppState;

// Orderbook WebSocket  GET /ws/:market_id
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, market_id))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, market_id: i32) {
    let intial_arc = {
        if let Ok(map) = state.orderbook.read() {
             map.get(&market_id).map(Arc::clone)
        } else {
            None
        }
    };

    match intial_arc {
        Some(value) => {
            let msg = serde_json::to_string(&(*value).clone()).unwrap();
            socket.send(Message::Text(msg)).await.ok();
        },
        None => log::info!("No orderbook snapshot for market_id={}", market_id)
    }

    let mut rx = {
        let rx_opt = {
            let ch = state.ob_channels.read().unwrap();
            ch.get(&market_id).map(|s| s.subscribe())
        };
        match rx_opt {
            Some(rx) => rx,
            None => {
                // Market exists but wasn't bootstrapped (created after startup), create channel lazily
                let (tx, rx) = tokio::sync::broadcast::channel(256);
                state.ob_channels.write().unwrap().insert(market_id, tx);
                rx
            }
        }
    };

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(diff) => {
                        let json = serde_json::to_string(&diff).unwrap();
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        let json = {
                            let ob = state.orderbook.read().unwrap();
                            ob.get(&market_id).map(|snap| serde_json::to_string(&**snap).unwrap())
                        };
                        if let Some(json) = json {
                            socket.send(Message::Text(json)).await.ok();
                        }
                    }
                    Err(RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => {
                if msg.is_none() { break; }
            }
        }
    }
}

// Price WebSocket  GET /ws/price/:market_id?token=yes
// Sends one message on every trade:
//   { "timestamp": 1711929600, "price": 6500 }
//
// Frontend just appends each point to the line chart.

#[derive(Deserialize)]
pub struct PriceWsParams {
    /// "yes" or "no" which token to watch (default is yes)
    pub token: Option<String>,
}

#[derive(Serialize)]
struct PricePoint {
    timestamp: i64,
    price:     i64,
}

pub async fn price_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
    Query(params): Query<PriceWsParams>,
) -> impl IntoResponse {
    let token = params.token.unwrap_or_else(|| "yes".to_string());
    ws.on_upgrade(move |socket| handle_price_socket(socket, state, market_id, token))
}

async fn handle_price_socket(
    mut socket: WebSocket,
    state: AppState,
    market_id: i32,
    token: String,
) {
    // Subscribe to trade ticks for this market
    let mut rx = {
        let rx_opt = {
            let ch = state.trade_channels.read().unwrap();
            ch.get(&market_id).map(|s| s.subscribe())
        };
        match rx_opt {
            Some(rx) => rx,
            None => {
                let (tx, rx) = tokio::sync::broadcast::channel(256);
                state.trade_channels.write().unwrap().insert(market_id, tx);
                rx
            }
        }
    };

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(tick) => {
                        // Only forward ticks for the token this client wants
                        if tick.token_type != token {
                            continue;
                        }
                        let point = PricePoint {
                            timestamp: tick.event_timestamp,
                            price:     tick.price,
                        };
                        let json = serde_json::to_string(&point).unwrap();
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue, // just skip, next tick will come
                    Err(RecvError::Closed)    => break,
                }
            }
            msg = socket.recv() => {
                if msg.is_none() { break; }
            }
        }
    }
}
