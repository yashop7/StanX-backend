use std::sync::Arc;

use anyhow::{Context, Result};
use common::{OrderbookDiff, OrderbookState, OrderbookWrite, TradeTick};
use log::{error, warn, info};
use tokio::sync::broadcast;

use crate::state::state::AppState;

pub async fn bootstrap(state: &AppState) -> Result<()> {
    let db = &state.db;

    let markets = db
        .get_active_markets()
        .await
        .context("Failed to fetch active markets")?;

    let total = markets.len();

    for market in markets {
        let market_id = market.market_id;

        let market_info = match db.get_market(market_id).await {
            Ok(Some(info)) => info,
            Ok(None) => { warn!("Market {} not found, skipping", market_id); continue; }
            Err(e) => { error!("Failed to load market {}: {:#}", market_id, e); continue; }
        };

        let mut orderbook = OrderbookState::new(0, market_id);
        if let Ok(snapshot) = db.get_orderbook(market_id).await {
            orderbook.push_snapshot(snapshot);
        }

        if let Ok(mut guard) = state.orderbook.write() {
            guard.insert(market_info.market_id, Arc::new(orderbook));
        }

        let (ob_tx, _) = broadcast::channel::<OrderbookDiff>(256);
        if let Ok(mut guard) = state.ob_channels.write() {
            guard.insert(market_info.market_id, ob_tx);
        }

        let (trade_tx, _) = broadcast::channel::<TradeTick>(256);
        if let Ok(mut guard) = state.trade_channels.write() {
            guard.insert(market_info.market_id, trade_tx);
        }
    }

    info!("Bootstrap complete: {} markets loaded", total);
    Ok(())
}
