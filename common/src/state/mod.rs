use std::{collections::HashMap, sync::Arc};

use db::models::events::LiveOrder;
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
    pub fn new(&mut self) -> Self {
        Self {
            slot: 0,
            market_id: 0,
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

    pub fn remove(&mut self, order_id: i64) {
        self.yes_bids.retain(|o| o.order_id != order_id);
        self.yes_asks.retain(|o| o.order_id != order_id);
        self.no_bids.retain(|o| o.order_id != order_id);
        self.no_asks.retain(|o| o.order_id != order_id);
    }
}

#[derive(Serialize, Deserialize)]
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
    pub fn new(&mut self) -> Self {
        Self {
            slot: 0,
            market_id: 0,
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
