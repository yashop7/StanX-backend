use anchor_lang::prelude::*;

use crate::state::*;

#[event]
pub struct MarketInitialized {
    pub market_id: u32,
    pub authority: Pubkey,
    pub settlement_deadline: i64,
    pub collateral_mint: Pubkey,
    pub outcome_yes_mint: Pubkey,
    pub outcome_no_mint: Pubkey,
    pub meta_data_url: String,
    pub timestamp: i64,
}

#[event]
pub struct TokensSplit {
    pub market_id: u32,
    pub user: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct TokensMerged {
    pub market_id: u32,
    pub user: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct OrderPlaced {
    pub market_id: u32,
    pub order_id: u64,
    pub user: Pubkey,
    pub side: OrderSide,
    pub token_type: TokenType,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}
#[event]
pub struct RewardsClaimed {
    pub market_id: u32,
    pub user: Pubkey,
    pub collateral_amount: u64,
    pub yes_tokens_burned: u64,
    pub no_tokens_burned: u64,
    pub timestamp: i64,
}

#[event]
pub struct MarketOrderExecuted {
    pub market_id: u32,
    pub user: Pubkey,
    pub side: OrderSide,
    pub token_type: TokenType,
    pub initial_quantity: u64,
    pub filled_quantity: u64,
    pub orders_matched: u64,
    pub timestamp: i64,
}

#[event]
pub struct OrderCancelled {
    pub market_id: u32,
    pub order_id: u64,
    pub user: Pubkey,
    pub side: OrderSide,
    pub token_type: TokenType,
    pub remaining_quantity: u64,
    pub timestamp: i64,
}

#[event]
pub struct FundsClaimed {
    pub market_id: u32,
    pub user: Pubkey,
    pub collateral_amount: u64,
    pub yes_amount: u64,
    pub no_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct MetadataUpdated {
    pub market_id: u32,
    pub authority: Pubkey,
    pub new_metadata_url: String,
    pub timestamp: i64,
}

#[event]
pub struct MarketClosed {
    pub market_id: u32,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct WinningSideSet {
    pub market_id: u32,
    pub winning_outcome: WinningOutcome,
    pub authority: Pubkey,
    pub timestamp: i64,
}

// For market orders taker_order_id: 0 , it's zero bcoz market orders never rest on the book so they have no order_id.
#[event]
pub struct OrderMatched {
    pub market_id: u32,
    pub taker_order_id: u64,
    pub maker_order_id: u64,
    pub taker_side: OrderSide,
    pub taker: Pubkey,
    pub maker: Pubkey,
    pub token_type: TokenType,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}