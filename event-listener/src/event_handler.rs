use anchor_lang::AnchorDeserialize;
use anyhow::Result;
use db::Db;
use db::models::events::{OrderSide, TokenType, WinningOutcome};

use crate::types::*;

const DISC_MARKET_INITIALIZED: [u8; 8]    = [134, 160, 122,  87,  50,   3, 255,  81];
const DISC_ORDER_PLACED: [u8; 8]          = [ 96, 130, 204, 234, 169, 219, 216, 227];
const DISC_ORDER_MATCHED: [u8; 8]         = [211,   0, 178, 174,  61, 245,  45, 250];
const DISC_ORDER_CANCELLED: [u8; 8]       = [108,  56, 128,  68, 168, 113, 168, 239];
const DISC_MARKET_ORDER_EXECUTED: [u8; 8] = [ 26, 218, 209, 198, 109, 184, 206, 124];
const DISC_TOKENS_SPLIT: [u8; 8]          = [203, 162, 194, 220, 152,  63,  72,  37];
const DISC_TOKENS_MERGED: [u8; 8]         = [178, 179, 160,  81, 141, 143, 120,  66];
const DISC_WINNING_SIDE_SET: [u8; 8]      = [ 21, 118,  98,  26, 169, 108,  60,  42];
const DISC_REWARDS_CLAIMED: [u8; 8]       = [ 75,  98,  88,  18, 219, 112,  88, 121];
const DISC_MARKET_CLOSED: [u8; 8]         = [ 86,  91, 119,  43,  94,   0, 217, 113];
const DISC_METADATA_UPDATED: [u8; 8]      = [132,  36, 215, 246, 166,  90, 189,  44];
const DISC_FUNDS_CLAIMED: [u8; 8]         = [202, 115, 101, 227,  91, 111, 239, 217];


/// Decode and persist a single base64-decoded program-data payload.
/// The first 8 bytes are the Anchor event discriminator; the rest is Borsh data.
pub async fn handle_event(sig: &str, slot: u64, data: &[u8], db: &Db) -> Result<()> {
    if data.len() < 8 {
        return Ok(());
    }
    let disc = data[..8].try_into().unwrap();
    let payload = &mut &data[8..];
    let slot = slot as i64;

    match disc {
        DISC_MARKET_INITIALIZED => {
            let ev = MarketInitialized::deserialize(payload)?;
            log::info!("MarketInitialized: market_id={} sig={}", ev.market_id, sig);
            db.store_market_initialized(
                sig,
                slot,
                ev.market_id as i32,
                &ev.authority.to_string(),
                ev.settlement_deadline,
                &ev.collateral_mint.to_string(),
                &ev.outcome_yes_mint.to_string(),
                &ev.outcome_no_mint.to_string(),
                &ev.meta_data_url,
                ev.timestamp,
            )
            .await?;
        }
        DISC_ORDER_PLACED => {
            let ev = OrderPlaced::deserialize(payload)?;
            log::info!("OrderPlaced: market={} order={} sig={}", ev.market_id, ev.order_id, sig);
            db.store_order_placed(
                sig,
                slot,
                ev.market_id as i32,
                ev.order_id as i64,
                &ev.user.to_string(),
                map_side(&ev.side),
                map_token_type(&ev.token_type),
                ev.price as i64,
                ev.quantity as i64,
                ev.timestamp,
            )
            .await?;
        }
        DISC_ORDER_MATCHED => {
            let ev = OrderMatched::deserialize(payload)?;
            log::info!("OrderMatched: market={} maker_order={} sig={}", ev.market_id, ev.maker_order_id, sig);
            db.store_order_matched(
                sig,
                slot,
                ev.market_id as i32,
                ev.maker_order_id as i64,
                map_side(&ev.taker_side),
                &ev.taker.to_string(),
                &ev.maker.to_string(),
                map_token_type(&ev.token_type),
                ev.price as i64,
                ev.quantity as i64,
                ev.timestamp,
            )
            .await?;
        }
        DISC_ORDER_CANCELLED => {
            let ev = OrderCancelled::deserialize(payload)?;
            log::info!("OrderCancelled: market={} order={} sig={}", ev.market_id, ev.order_id, sig);
            db.store_order_cancelled(
                sig,
                slot,
                ev.market_id as i32,
                ev.order_id as i64,
                &ev.user.to_string(),
                map_side(&ev.side),
                map_token_type(&ev.token_type),
                ev.remaining_quantity as i64,
                ev.timestamp,
            )
            .await?;
        }
        DISC_MARKET_ORDER_EXECUTED => {
            let ev = MarketOrderExecuted::deserialize(payload)?;
            log::info!("MarketOrderExecuted: market={} sig={}", ev.market_id, sig);
            db.store_market_order_executed(
                sig,
                slot,
                ev.market_id as i32,
                &ev.user.to_string(),
                map_side(&ev.side),
                map_token_type(&ev.token_type),
                ev.total_quantity as i64,
                ev.orders_matched as i64,
                ev.timestamp,
            )
            .await?;
        }
        DISC_TOKENS_SPLIT => {
            let ev = TokensSplit::deserialize(payload)?;
            log::info!("TokensSplit: market={} sig={}", ev.market_id, sig);
            db.store_tokens_split(
                sig,
                slot,
                ev.market_id as i32,
                &ev.user.to_string(),
                ev.amount as i64,
                ev.timestamp,
            )
            .await?;
        }
        DISC_TOKENS_MERGED => {
            let ev = TokensMerged::deserialize(payload)?;
            log::info!("TokensMerged: market={} sig={}", ev.market_id, sig);
            db.store_tokens_merged(
                sig,
                slot,
                ev.market_id as i32,
                &ev.user.to_string(),
                ev.amount as i64,
                ev.timestamp,
            )
            .await?;
        }
        DISC_WINNING_SIDE_SET => {
            let ev = WinningSideSet::deserialize(payload)?;
            log::info!("WinningSideSet: market={} outcome={:?} sig={}", ev.market_id, ev.winning_outcome, sig);
            db.store_winning_side_set(
                sig,
                slot,
                ev.market_id as i32,
                map_winning_outcome(&ev.winning_outcome),
                &ev.authority.to_string(),
                ev.timestamp,
            )
            .await?;
        }
        DISC_REWARDS_CLAIMED => {
            let ev = RewardsClaimed::deserialize(payload)?;
            log::info!("RewardsClaimed: market={} sig={}", ev.market_id, sig);
            db.store_rewards_claimed(
                sig,
                slot,
                ev.market_id as i32,
                &ev.user.to_string(),
                ev.collateral_amount as i64,
                ev.yes_tokens_burned as i64,
                ev.no_tokens_burned as i64,
                ev.timestamp,
            )
            .await?;
        }
        DISC_MARKET_CLOSED => {
            let ev = MarketClosed::deserialize(payload)?;
            log::info!("MarketClosed: market={} sig={}", ev.market_id, sig);
            db.store_market_closed(
                sig,
                slot,
                ev.market_id as i32,
                &ev.authority.to_string(),
                ev.timestamp,
            )
            .await?;
        }
        DISC_METADATA_UPDATED => {
            let ev = MetadataUpdated::deserialize(payload)?;
            log::info!("MetadataUpdated: market={} sig={}", ev.market_id, sig);
            db.store_metadata_updated(
                sig,
                slot,
                ev.market_id as i32,
                &ev.authority.to_string(),
                &ev.new_metadata_url,
                ev.timestamp,
            )
            .await?;
        }
        DISC_FUNDS_CLAIMED => {
            let ev = FundsClaimed::deserialize(payload)?;
            log::info!("FundsClaimed: market={} user={} sig={}", ev.market_id, ev.user, sig);
            db.store_funds_claimed(
                sig,
                slot,
                ev.market_id as i32,
                &ev.user.to_string(),
                ev.collateral_amount as i64,
                ev.yes_amount as i64,
                ev.no_amount as i64,
                ev.timestamp,
            )
            .await?;
        }
        _ => {
            log::debug!("Unknown event discriminator {:?} sig={}", disc, sig);
        }
    }

    // Update indexer cursor after every successful event
    db.update_cursor(sig, slot).await?;

    Ok(())
}

// Mapping event-listener enum types to DB enum types
fn map_side(side: &crate::types::OrderSide) -> OrderSide {
    match side {
        crate::types::OrderSide::Buy => OrderSide::Buy,
        crate::types::OrderSide::Sell => OrderSide::Sell,
    }
}

fn map_token_type(tt: &crate::types::TokenType) -> TokenType {
    match tt {
        crate::types::TokenType::Yes => TokenType::Yes,
        crate::types::TokenType::No => TokenType::No,
    }
}

fn map_winning_outcome(wo: &crate::types::WinningOutcome) -> WinningOutcome {
    match wo {
        crate::types::WinningOutcome::OutcomeA => WinningOutcome::OutcomeA,
        crate::types::WinningOutcome::OutcomeB => WinningOutcome::OutcomeB,
        crate::types::WinningOutcome::Neither => WinningOutcome::Neither,
    }
}
