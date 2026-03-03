use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use solana_pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::str::FromStr;

// Event discriminators (first 8 bytes of event data)
// These are calculated as the first 8 bytes of sha256("event:EventName")
const MARKET_INITIALIZED_DISCRIMINATOR: [u8; 8] = [254, 200, 123, 61, 26, 220, 26, 74];
const ORDER_PLACED_DISCRIMINATOR: [u8; 8] = [52, 244, 203, 215, 222, 189, 95, 157];
const ORDER_MATCHED_DISCRIMINATOR: [u8; 8] = [249, 63, 176, 30, 181, 99, 145, 139];
const ORDER_CANCELLED_DISCRIMINATOR: [u8; 8] = [243, 44, 86, 54, 177, 189, 111, 137];
const MARKET_ORDER_EXECUTED_DISCRIMINATOR: [u8; 8] = [98, 129, 91, 38, 122, 245, 201, 101];
const TOKENS_SPLIT_DISCRIMINATOR: [u8; 8] = [127, 208, 11, 70, 114, 221, 168, 193];
const TOKENS_MERGED_DISCRIMINATOR: [u8; 8] = [92, 21, 136, 76, 141, 205, 128, 130];
const WINNING_SIDE_SET_DISCRIMINATOR: [u8; 8] = [56, 229, 36, 197, 66, 212, 113, 121];
const REWARDS_CLAIMED_DISCRIMINATOR: [u8; 8] = [173, 110, 208, 136, 176, 248, 4, 249];
const MARKET_CLOSED_DISCRIMINATOR: [u8; 8] = [98, 177, 229, 54, 157, 199, 105, 149];
const METADATA_UPDATED_DISCRIMINATOR: [u8; 8] = [234, 74, 180, 50, 78, 208, 161, 150];

#[derive(Debug, Clone)]
pub enum PredictionMarketEvent {
    MarketInitialized(MarketInitializedEvent),
    OrderPlaced(OrderPlacedEvent),
    OrderMatched(OrderMatchedEvent),
    OrderCancelled(OrderCancelledEvent),
    MarketOrderExecuted(MarketOrderExecutedEvent),
    TokensSplit(TokensSplitEvent),
    TokensMerged(TokensMergedEvent),
    WinningSideSet(WinningSideSetEvent),
    RewardsClaimed(RewardsClaimedEvent),
    MarketClosed(MarketClosedEvent),
    MetadataUpdated(MetadataUpdatedEvent),
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct MarketInitializedEvent {
    pub market_id: u32,
    pub authority: Pubkey,
    pub settlement_deadline: i64,
    pub collateral_mint: Pubkey,
    pub outcome_yes_mint: Pubkey,
    pub outcome_no_mint: Pubkey,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct OrderPlacedEvent {
    pub market_id: u32,
    pub order_id: u64,
    pub user: Pubkey,
    pub side: OrderSide,
    pub token_type: TokenType,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct OrderMatchedEvent {
    pub market_id: u32,
    pub taker_order_id: u64,
    pub maker_order_id: u64,
    pub taker: Pubkey,
    pub maker: Pubkey,
    pub token_type: TokenType,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct OrderCancelledEvent {
    pub market_id: u32,
    pub order_id: u64,
    pub user: Pubkey,
    pub side: OrderSide,
    pub token_type: TokenType,
    pub remaining_quantity: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct MarketOrderExecutedEvent {
    pub market_id: u32,
    pub user: Pubkey,
    pub side: OrderSide,
    pub token_type: TokenType,
    pub total_quantity: u64,
    pub orders_matched: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct TokensSplitEvent {
    pub market_id: u32,
    pub user: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct TokensMergedEvent {
    pub market_id: u32,
    pub user: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct WinningSideSetEvent {
    pub market_id: u32,
    pub winning_outcome: WinningOutcome,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct RewardsClaimedEvent {
    pub market_id: u32,
    pub user: Pubkey,
    pub collateral_amount: u64,
    pub yes_tokens_burned: u64,
    pub no_tokens_burned: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct MarketClosedEvent {
    pub market_id: u32,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct MetadataUpdatedEvent {
    pub market_id: u32,
    pub authority: Pubkey,
    pub new_metadata_url: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub enum TokenType {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub enum WinningOutcome {
    OutcomeA,
    OutcomeB,
    Neither,
}

pub fn parse_event(data: &[u8]) -> Result<PredictionMarketEvent> {
    if data.len() < 8 {
        return Err(anyhow::anyhow!("Event data too short"));
    }

    let (discriminator, event_data) = data.split_at(8);
    let discriminator: [u8; 8] = discriminator.try_into()?;

    match discriminator {
        MARKET_INITIALIZED_DISCRIMINATOR => {
            let event = MarketInitializedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize MarketInitialized event")?;
            Ok(PredictionMarketEvent::MarketInitialized(event))
        }
        ORDER_PLACED_DISCRIMINATOR => {
            let event = OrderPlacedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize OrderPlaced event")?;
            Ok(PredictionMarketEvent::OrderPlaced(event))
        }
        ORDER_MATCHED_DISCRIMINATOR => {
            let event = OrderMatchedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize OrderMatched event")?;
            Ok(PredictionMarketEvent::OrderMatched(event))
        }
        ORDER_CANCELLED_DISCRIMINATOR => {
            let event = OrderCancelledEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize OrderCancelled event")?;
            Ok(PredictionMarketEvent::OrderCancelled(event))
        }
        MARKET_ORDER_EXECUTED_DISCRIMINATOR => {
            let event = MarketOrderExecutedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize MarketOrderExecuted event")?;
            Ok(PredictionMarketEvent::MarketOrderExecuted(event))
        }
        TOKENS_SPLIT_DISCRIMINATOR => {
            let event = TokensSplitEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize TokensSplit event")?;
            Ok(PredictionMarketEvent::TokensSplit(event))
        }
        TOKENS_MERGED_DISCRIMINATOR => {
            let event = TokensMergedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize TokensMerged event")?;
            Ok(PredictionMarketEvent::TokensMerged(event))
        }
        WINNING_SIDE_SET_DISCRIMINATOR => {
            let event = WinningSideSetEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize WinningSideSet event")?;
            Ok(PredictionMarketEvent::WinningSideSet(event))
        }
        REWARDS_CLAIMED_DISCRIMINATOR => {
            let event = RewardsClaimedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize RewardsClaimed event")?;
            Ok(PredictionMarketEvent::RewardsClaimed(event))
        }
        MARKET_CLOSED_DISCRIMINATOR => {
            let event = MarketClosedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize MarketClosed event")?;
            Ok(PredictionMarketEvent::MarketClosed(event))
        }
        METADATA_UPDATED_DISCRIMINATOR => {
            let event = MetadataUpdatedEvent::deserialize(&mut &event_data[..])
                .context("Failed to deserialize MetadataUpdated event")?;
            Ok(PredictionMarketEvent::MetadataUpdated(event))
        }
        _ => Err(anyhow::anyhow!("Unknown event discriminator: {:?}", discriminator)),
    }
}

pub fn print_event(event: &PredictionMarketEvent, signature: &str, slot: u64) {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  NEW EVENT DETECTED");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Signature: {}", signature);
    println!("║  Slot: {}", slot);
    println!("╠══════════════════════════════════════════════════════════════╣");
    
    match event {
        PredictionMarketEvent::MarketInitialized(e) => {
            println!("║  Event Type: MARKET_INITIALIZED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  Authority: {}", e.authority);
            println!("║  Settlement Deadline: {}", e.settlement_deadline);
            println!("║  Collateral Mint: {}", e.collateral_mint);
            println!("║  Outcome YES Mint: {}", e.outcome_yes_mint);
            println!("║  Outcome NO Mint: {}", e.outcome_no_mint);
        }
        PredictionMarketEvent::OrderPlaced(e) => {
            println!("║  Event Type: ORDER_PLACED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  Order ID: {}", e.order_id);
            println!("║  User: {}", e.user);
            println!("║  Side: {:?}", e.side);
            println!("║  Token Type: {:?}", e.token_type);
            println!("║  Price: {}", e.price);
            println!("║  Quantity: {}", e.quantity);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::OrderMatched(e) => {
            println!("║  Event Type: ORDER_MATCHED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  Taker Order ID: {}", e.taker_order_id);
            println!("║  Maker Order ID: {}", e.maker_order_id);
            println!("║  Taker: {}", e.taker);
            println!("║  Maker: {}", e.maker);
            println!("║  Token Type: {:?}", e.token_type);
            println!("║  Price: {}", e.price);
            println!("║  Quantity: {}", e.quantity);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::OrderCancelled(e) => {
            println!("║  Event Type: ORDER_CANCELLED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  Order ID: {}", e.order_id);
            println!("║  User: {}", e.user);
            println!("║  Side: {:?}", e.side);
            println!("║  Token Type: {:?}", e.token_type);
            println!("║  Remaining Quantity: {}", e.remaining_quantity);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::MarketOrderExecuted(e) => {
            println!("║  Event Type: MARKET_ORDER_EXECUTED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  User: {}", e.user);
            println!("║  Side: {:?}", e.side);
            println!("║  Token Type: {:?}", e.token_type);
            println!("║  Total Quantity: {}", e.total_quantity);
            println!("║  Orders Matched: {}", e.orders_matched);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::TokensSplit(e) => {
            println!("║  Event Type: TOKENS_SPLIT");
            println!("║  Market ID: {}", e.market_id);
            println!("║  User: {}", e.user);
            println!("║  Amount: {}", e.amount);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::TokensMerged(e) => {
            println!("║  Event Type: TOKENS_MERGED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  User: {}", e.user);
            println!("║  Amount: {}", e.amount);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::WinningSideSet(e) => {
            println!("║  Event Type: WINNING_SIDE_SET");
            println!("║  Market ID: {}", e.market_id);
            println!("║  Winning Outcome: {:?}", e.winning_outcome);
            println!("║  Authority: {}", e.authority);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::RewardsClaimed(e) => {
            println!("║  Event Type: REWARDS_CLAIMED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  User: {}", e.user);
            println!("║  Collateral Amount: {}", e.collateral_amount);
            println!("║  YES Tokens Burned: {}", e.yes_tokens_burned);
            println!("║  NO Tokens Burned: {}", e.no_tokens_burned);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::MarketClosed(e) => {
            println!("║  Event Type: MARKET_CLOSED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  Authority: {}", e.authority);
            println!("║  Timestamp: {}", e.timestamp);
        }
        PredictionMarketEvent::MetadataUpdated(e) => {
            println!("║  Event Type: METADATA_UPDATED");
            println!("║  Market ID: {}", e.market_id);
            println!("║  Authority: {}", e.authority);
            println!("║  New Metadata URL: {}", e.new_metadata_url);
            println!("║  Timestamp: {}", e.timestamp);
        }
    }
    
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
