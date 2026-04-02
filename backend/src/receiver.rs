use std::sync::Arc;

use crate::AppState;
use common::{ApplyDiff, OrderbookDiff, OrderbookState, TradeTick};
use futures_util::StreamExt;
use tokio::sync::broadcast;

pub async fn run(state: &AppState, redis_url: String) {
    let client = redis::Client::open(redis_url.clone()).expect("Failed to create Redis client");

    let mut conn = client
        .get_async_pubsub()
        .await
        .expect("Failed to connect to Redis pubsub");

    // Subscribe to both orderbook diffs and trade ticks
    conn.psubscribe("orderbook:market:*")
        .await
        .expect("Failed to subscribe to orderbook channels");
    conn.psubscribe("trades:market:*")
        .await
        .expect("Failed to subscribe to trades channels");

    log::info!("Redis subscriber listening on orderbook:market:* and trades:market:*");

    while let Some(msg) = conn.on_message().next().await {
        let channel = msg.get_channel_name().to_string();

        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get message payload: {}", e);
                continue;
            }
        };

        if channel.starts_with("orderbook:market:") {
            // ── Orderbook diff ────────────────────────────────────────────
            let diff: OrderbookDiff = match serde_json::from_str(&payload) {
                Ok(d) => d,
                Err(e) => {
                    log::error!("Failed to deserialize OrderbookDiff: {}", e);
                    continue;
                }
            };

            // Pointer-swap: clone old state, apply diff, swap in new Arc
            let old_arc: Arc<OrderbookState> = {
                let map = state.orderbook.read().unwrap();
                match map.get(&diff.market_id) {
                    Some(arc) => Arc::clone(arc),
                    None => Arc::new(OrderbookState::new(0, diff.market_id)),
                }
            };

            let mut new_snap: OrderbookState = (*old_arc).clone();
            diff.apply(&mut new_snap);

            {
                let mut map = state.orderbook.write().unwrap();
                map.insert(diff.market_id, Arc::new(new_snap));
            }

            if let Ok(ch) = state.ob_channels.read() {
                if let Some(tx) = ch.get(&diff.market_id) {
                    let _ = tx.send(diff);
                }
            }

        } else if channel.starts_with("trades:market:") {
            // ── Trade tick ────────────────────────────────────────────────
            let tick: TradeTick = match serde_json::from_str(&payload) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Failed to deserialize TradeTick: {}", e);
                    continue;
                }
            };

            // Create channel for this market lazily on first trade
            {
                let needs_channel = state.trade_channels.read().unwrap()
                    .get(&tick.market_id).is_none();
                if needs_channel {
                    let (tx, _) = broadcast::channel::<TradeTick>(256);
                    state.trade_channels.write().unwrap()
                        .insert(tick.market_id, tx);
                }
            }

            if let Ok(ch) = state.trade_channels.read() {
                if let Some(tx) = ch.get(&tick.market_id) {
                    let _ = tx.send(tick);
                }
            }
        }
    }

    log::error!("Redis pubsub stream ended — connection lost");
}
