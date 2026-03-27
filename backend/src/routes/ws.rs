use std::sync::Arc;

use axum::{
    extract::{Path, State},
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use axum::extract::ws::*;
use tokio::sync::broadcast::error::RecvError;

use crate::state::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, market_id))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, market_id: i32) {
    // Subscribe to the Particular Market Id
    // Then Send the Diff, which we are getting from the broadcast to the Subscribed user
    let intial_arc = { // Right now we have access to the Orderbook State in the clone form
        if let Ok(map) = state.orderbook.read() {
             map.get(&market_id).map(Arc::clone)
        } else {
            log::info!("Failed to read orderbook");
            None
        }
    };

    match intial_arc {
        Some(value) => {
            // Sending the Initial State
            let msg = serde_json::to_string(&(*value).clone()).unwrap();
            socket.send(Message::Text(msg)).await.ok();
        },
        None => log::info!("Failed to read orderbook")
    }

    // STEP 2: Subscribe to the broadcast channel for this market
    //         This gives THIS client its own receiver
    let mut rx = {
        let ch = state.ob_channels.read().unwrap();
        match ch.get(&market_id) {
            Some(sender) => sender.subscribe(),
            None => {
                // No channel exists yet, market has no orders, nothing to stream
                log::info!("No broadcast channel for market_id={}, closing ws", market_id);
                return;
            }
        }
    };

    // STEP 3: Loop — forward every diff to this client
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(diff) => {
                        let json = serde_json::to_string(&diff).unwrap();
                        if socket.send(Message::Text(json)).await.is_err() {
                            break; // client disconnected
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        // Client was too slow, missed some diffs
                        // Resync: send full snapshot again
                        // Lock is dropped before the .await to avoid holding a sync
                        // mutex guard across an async suspension point
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
                if msg.is_none() { break; } // client closed connection
            }
        }
    }
    // rx is dropped here — automatically unsubscribed
}
