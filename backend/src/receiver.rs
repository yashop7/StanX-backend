use std::sync::Arc;

use crate::AppState;
use common::{ApplyDiff, OrderbookDiff, OrderbookState, TradeTick};
use futures_util::StreamExt;
use tokio::sync::broadcast;

pub async fn run(state: &AppState, redis_url: String) {
    let mut backoff_secs: u64 = 1;

    loop {
        log::info!("Redis receiver: connecting to pubsub...");

        let client = match redis::Client::open(redis_url.clone()) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Redis receiver: invalid URL: {}. Retrying in {}s", e, backoff_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60);
                continue;
            }
        };

        let mut conn = match client.get_async_pubsub().await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Redis receiver: connect failed: {}. Retrying in {}s", e, backoff_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60);
                continue;
            }
        };

        if let Err(e) = conn.psubscribe("orderbook:market:*").await {
            log::error!("Redis receiver: psubscribe orderbook failed: {}. Retrying in {}s", e, backoff_secs);
            tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
            backoff_secs = (backoff_secs * 2).min(60);
            continue;
        }
        if let Err(e) = conn.psubscribe("trades:market:*").await {
            log::error!("Redis receiver: psubscribe trades failed: {}. Retrying in {}s", e, backoff_secs);
            tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
            backoff_secs = (backoff_secs * 2).min(60);
            continue;
        }

        log::info!("Redis receiver: listening on orderbook:market:* and trades:market:*");
        backoff_secs = 1;

        let mut stream = conn.on_message();
        while let Some(msg) = stream.next().await {
            let channel = msg.get_channel_name().to_string();

            let payload: String = match msg.get_payload() {
                Ok(p) => p,
                Err(e) => {
                    log::error!("Failed to get message payload: {}", e);
                    continue;
                }
            };

            if channel.starts_with("orderbook:market:") {
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
                let tick: TradeTick = match serde_json::from_str(&payload) {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("Failed to deserialize TradeTick: {}", e);
                        continue;
                    }
                };

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

        log::error!("Redis receiver: pubsub stream ended — reconnecting in {}s", backoff_secs);
        tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(60);
    }
}
