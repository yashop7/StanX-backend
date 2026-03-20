use std::{collections::HashMap, sync::Arc};

use db::models::events::{LiveOrder, OrderbookResponse};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize)]
pub struct OrderbookState {
    pub slot: u64,
    pub market_id: i32,
    pub yes_bids: Vec<LiveOrder>, // sorted price desc
    pub yes_asks: Vec<LiveOrder>, // sorted price asc
    pub no_bids: Vec<LiveOrder>,
    pub no_asks: Vec<LiveOrder>,
}

impl OrderbookState {
    pub fn new(slot: u64, market_id: i32) -> Self {
        Self {
            slot,
            market_id,
            yes_bids: Vec::new(),
            yes_asks: Vec::new(),
            no_bids: Vec::new(),
            no_asks: Vec::new(),
        }
    }

    pub fn push(&mut self, token_type: TokenType, order_side: OrderSide, order: LiveOrder) {
        match (token_type, order_side) {
            (TokenType::Yes, OrderSide::Buy) => {
                self.yes_bids.push(order);
                self.yes_bids
                    .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()); // desc
            }
            (TokenType::Yes, OrderSide::Sell) => {
                self.yes_asks.push(order);
                self.yes_asks
                    .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()); // asc
            }
            (TokenType::No, OrderSide::Buy) => {
                self.no_bids.push(order);
                self.no_bids
                    .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()); // desc
            }
            (TokenType::No, OrderSide::Sell) => {
                self.no_asks.push(order);
                self.no_asks
                    .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()); // asc
            }
        }
    }

    pub fn push_orderbook_response(&mut self, orderbook: OrderbookResponse) {

        self.yes_bids = orderbook.yes_buy_orders;
        self.yes_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()); // desc

        self.yes_asks = orderbook.yes_sell_orders;
        self.yes_asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()); // asc

        self.no_bids = orderbook.no_buy_orders;
        self.no_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()); // desc

        self.no_asks = orderbook.no_sell_orders;
        self.no_asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()); // asc
    }

    pub fn remove(&mut self, order_id: i64) {
        self.yes_bids.retain(|o| o.order_id != order_id);
        self.yes_asks.retain(|o| o.order_id != order_id);
        self.no_bids.retain(|o| o.order_id != order_id);
        self.no_asks.retain(|o| o.order_id != order_id);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrderbookDiff {
    pub slot: u64,
    pub market_id: i32,
    pub yes_bids_added: Vec<LiveOrder>,
    pub yes_bids_removed: Vec<i64>,
    pub yes_asks_added: Vec<LiveOrder>,
    pub yes_asks_removed: Vec<i64>,
    pub no_bids_added: Vec<LiveOrder>,
    pub no_bids_removed: Vec<i64>,
    pub no_asks_added: Vec<LiveOrder>,
    pub no_asks_removed: Vec<i64>,
}

impl OrderbookDiff {
    pub fn new(slot: u64, market_id: i32) -> Self {
        Self {
            slot,
            market_id,
            yes_bids_added: Vec::new(),
            yes_bids_removed: Vec::new(),
            yes_asks_added: Vec::new(),
            yes_asks_removed: Vec::new(),
            no_bids_added: Vec::new(),
            no_bids_removed: Vec::new(),
            no_asks_added: Vec::new(),
            no_asks_removed: Vec::new(),
        }
    }

    /// Compute the diff between two orderbook snapshots.
    pub fn from_states(old: &OrderbookState, new: &OrderbookState) -> Self {
        let mut diff = Self::new(new.slot, new.market_id);
        diff_side(&old.yes_bids, &new.yes_bids, &mut diff.yes_bids_added, &mut diff.yes_bids_removed);
        diff_side(&old.yes_asks, &new.yes_asks, &mut diff.yes_asks_added, &mut diff.yes_asks_removed);
        diff_side(&old.no_bids,  &new.no_bids,  &mut diff.no_bids_added,  &mut diff.no_bids_removed);
        diff_side(&old.no_asks,  &new.no_asks,  &mut diff.no_asks_added,  &mut diff.no_asks_removed);
        diff
    }

    /// Apply this diff to an existing orderbook state in place.
    pub fn apply(&self, state: &mut OrderbookState) {
        for &id in &self.yes_bids_removed { state.yes_bids.retain(|o| o.order_id != id); }
        for &id in &self.yes_asks_removed { state.yes_asks.retain(|o| o.order_id != id); }
        for &id in &self.no_bids_removed  { state.no_bids.retain(|o| o.order_id != id); }
        for &id in &self.no_asks_removed  { state.no_asks.retain(|o| o.order_id != id); }

        for order in &self.yes_bids_added { state.push(TokenType::Yes, OrderSide::Buy,  order.clone()); }
        for order in &self.yes_asks_added { state.push(TokenType::Yes, OrderSide::Sell, order.clone()); }
        for order in &self.no_bids_added  { state.push(TokenType::No,  OrderSide::Buy,  order.clone()); }
        for order in &self.no_asks_added  { state.push(TokenType::No,  OrderSide::Sell, order.clone()); }

        state.slot = self.slot;
    }

    /// Returns true if this diff contains no changes.
    pub fn is_empty(&self) -> bool {
        self.yes_bids_added.is_empty()
            && self.yes_bids_removed.is_empty()
            && self.yes_asks_added.is_empty()
            && self.yes_asks_removed.is_empty()
            && self.no_bids_added.is_empty()
            && self.no_bids_removed.is_empty()
            && self.no_asks_added.is_empty()
            && self.no_asks_removed.is_empty()
    }
}

fn diff_side(
    old: &[LiveOrder],
    new: &[LiveOrder],
    added: &mut Vec<LiveOrder>,
    removed: &mut Vec<i64>,
) {
    let old_map: HashMap<i64, &LiveOrder> = old.iter().map(|o| (o.order_id, o)).collect();
    let new_ids: std::collections::HashSet<i64> = new.iter().map(|o| o.order_id).collect();

    for order in new {
        match old_map.get(&order.order_id) {
            None => added.push(order.clone()),
            Some(old_order) if old_order.remaining_quantity != order.remaining_quantity => {
                removed.push(order.order_id);
                added.push(order.clone());
            }
            _ => {}
        }
    }
    // Order gone entirely (filled or cancelled) → remove
    for order in old {
        if !new_ids.contains(&order.order_id) {
            removed.push(order.order_id);
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TokenType {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WinningOutcome {
    OutcomeA,
    OutcomeB,
    Neither,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MarketStatus {
    Active,
    Settled,
    Closed,
}
