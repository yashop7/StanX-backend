use crate::types::*;
use anchor_lang::AnchorDeserialize;
use anyhow::Result;
use common::{OrderbookDiff, TradeTick};
use db::models::events::{LiveOrder, OrderSide, OrderStatus, TokenType, WinningOutcome};
use db::Db;
use redis::Commands;
use std::env;

const DISC_MARKET_INITIALIZED: [u8; 8] = [134, 160, 122, 87, 50, 3, 255, 81];
const DISC_ORDER_PLACED: [u8; 8] = [96, 130, 204, 234, 169, 219, 216, 227];
const DISC_ORDER_MATCHED: [u8; 8] = [211, 0, 178, 174, 61, 245, 45, 250];
const DISC_ORDER_CANCELLED: [u8; 8] = [108, 56, 128, 68, 168, 113, 168, 239];
const DISC_MARKET_ORDER_EXECUTED: [u8; 8] = [26, 218, 209, 198, 109, 184, 206, 124];
const DISC_TOKENS_SPLIT: [u8; 8] = [203, 162, 194, 220, 152, 63, 72, 37];
const DISC_TOKENS_MERGED: [u8; 8] = [178, 179, 160, 81, 141, 143, 120, 66];
const DISC_WINNING_SIDE_SET: [u8; 8] = [21, 118, 98, 26, 169, 108, 60, 42];
const DISC_REWARDS_CLAIMED: [u8; 8] = [75, 98, 88, 18, 219, 112, 88, 121];
const DISC_MARKET_CLOSED: [u8; 8] = [86, 91, 119, 43, 94, 0, 217, 113];
const DISC_METADATA_UPDATED: [u8; 8] = [132, 36, 215, 246, 166, 90, 189, 44];
const DISC_FUNDS_CLAIMED: [u8; 8] = [202, 115, 101, 227, 91, 111, 239, 217];

/// Decode and persist a single base64-decoded program-data payload.
/// The first 8 bytes are the Anchor event discriminator; the rest is Borsh data.
pub async fn handle_event(sig: &str, slot: u64, data: &[u8], db: &Db) -> Result<()> {
    let redis_port =
        env::var("REDIS_PORT").map_err(|_| anyhow::anyhow!("REDIS_PORT not set in environment"))?;
    let redis_address = env::var("REDIS_ADDRESS")
        .map_err(|_| anyhow::anyhow!("REDIS_ADDRESS not set in environment"))?;
    let redis_url = format!("redis://{}:{}", redis_address, redis_port);

    let redis_client = redis::Client::open(redis_url.clone())
        .map_err(|e| anyhow::anyhow!("Failed to connect to Redis at {}: {}", redis_url, e))?;

    let mut publisher = redis_client.get_connection()?;

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
            log::info!(
                "OrderPlaced: market={} order={} sig={}",
                ev.market_id,
                ev.order_id,
                sig
            );

            if let Err(e) = db.store_order_placed(
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
            .await
            {
                // FK violation (code 23503) means the market was never indexed.
                // This happens when the indexer was down during MarketInitialized.
                // Skip this order — it will be recovered by the backfill on next restart.
                let err_str = e.to_string();
                let is_fk_violation = err_str.contains("23503")
                    || err_str.contains("foreign key constraint");

                if is_fk_violation {
                    log::warn!(
                        "Skipping OrderPlaced sig={} market={} order={}: \
                         market not in DB (missed MarketInitialized — run backfill)",
                        sig, ev.market_id, ev.order_id
                    );
                    return Ok(());
                }
                return Err(e);
            }

            // Build the LiveOrder and push it into the correct side of the diff
            let order = LiveOrder {
                order_id: ev.order_id as i64,
                market_id: ev.market_id as i32,
                user_pubkey: ev.user.to_string(),
                side: map_side(&ev.side),
                token_type: map_token_type(&ev.token_type),
                price: ev.price as i64,
                original_quantity: ev.quantity as i64,
                remaining_quantity: ev.quantity as i64,
                status: OrderStatus::Open,
                placed_at: ev.timestamp,
                updated_at: chrono::Utc::now(),
            };

            let mut diff = OrderbookDiff::new(slot as u64, ev.market_id as i32);
            match (&ev.token_type, &ev.side) {
                (crate::types::TokenType::Yes, crate::types::OrderSide::Buy) => {
                    diff.yes_bids_added.push(order)
                }
                (crate::types::TokenType::Yes, crate::types::OrderSide::Sell) => {
                    diff.yes_asks_added.push(order)
                }
                (crate::types::TokenType::No, crate::types::OrderSide::Buy) => {
                    diff.no_bids_added.push(order)
                }
                (crate::types::TokenType::No, crate::types::OrderSide::Sell) => {
                    diff.no_asks_added.push(order)
                }
            }
            publish_diff(&mut publisher, ev.market_id as i32, &diff)?;
        }

        DISC_ORDER_MATCHED => {
            let ev = OrderMatched::deserialize(payload)?;
            log::info!(
                "OrderMatched: market={} maker_order={} sig={}",
                ev.market_id,
                ev.maker_order_id,
                sig
            );

            db.store_order_matched(
                sig,
                slot,
                ev.market_id as i32,
                ev.maker_order_id as i64,
                ev.taker_order_id as i64,
                map_side(&ev.taker_side),
                &ev.taker.to_string(),
                &ev.maker.to_string(),
                map_token_type(&ev.token_type),
                ev.price as i64,
                ev.quantity as i64,
                ev.timestamp,
            )
            .await?;

            let mut diff = OrderbookDiff::new(slot as u64, ev.market_id as i32);

            // The maker is on the opposite side of the taker.
            // taker_side=Buy  → maker was selling → maker lives in *_asks
            // taker_side=Sell → maker was buying  → maker lives in *_bids
            if let Ok(Some(updated)) = db
                .get_live_order(ev.market_id as i32, ev.maker_order_id as i64)
                .await
            {
                if updated.status == OrderStatus::PartiallyFilled {
                    // Remove the old entry first, then re-add with updated remaining_quantity.
                    // Without the remove, the in-memory orderbook would have two entries
                    // for the same order_id (old qty + new qty).
                    match (&ev.token_type, &ev.taker_side) {
                        (crate::types::TokenType::Yes, crate::types::OrderSide::Buy) => {
                            diff.yes_asks_removed.push(ev.maker_order_id as i64);
                            diff.yes_asks_added.push(updated)
                        }
                        (crate::types::TokenType::Yes, crate::types::OrderSide::Sell) => {
                            diff.yes_bids_removed.push(ev.maker_order_id as i64);
                            diff.yes_bids_added.push(updated)
                        }
                        (crate::types::TokenType::No, crate::types::OrderSide::Buy) => {
                            diff.no_asks_removed.push(ev.maker_order_id as i64);
                            diff.no_asks_added.push(updated)
                        }
                        (crate::types::TokenType::No, crate::types::OrderSide::Sell) => {
                            diff.no_bids_removed.push(ev.maker_order_id as i64);
                            diff.no_bids_added.push(updated)
                        }
                    }
                } else if updated.status == OrderStatus::Filled {
                    match (&ev.token_type, &ev.taker_side) {
                        (crate::types::TokenType::Yes, crate::types::OrderSide::Buy) => {
                            diff.yes_asks_removed.push(ev.maker_order_id as i64)
                        }
                        (crate::types::TokenType::Yes, crate::types::OrderSide::Sell) => {
                            diff.yes_bids_removed.push(ev.maker_order_id as i64)
                        }
                        (crate::types::TokenType::No, crate::types::OrderSide::Buy) => {
                            diff.no_asks_removed.push(ev.maker_order_id as i64)
                        }
                        (crate::types::TokenType::No, crate::types::OrderSide::Sell) => {
                            diff.no_bids_removed.push(ev.maker_order_id as i64)
                        }
                    }
                }
            }

            // Only limit-order takers (taker_order_id != 0) have a live_order entry.
            // Market-order takers (taker_order_id == 0) never rest on the book.
            // taker_side=Buy  → taker lives in *_bids
            // taker_side=Sell → taker lives in *_asks
            if ev.taker_order_id != 0 {
                if let Ok(Some(updated_taker)) = db
                    .get_live_order(ev.market_id as i32, ev.taker_order_id as i64)
                    .await
                {
                    if updated_taker.status == OrderStatus::PartiallyFilled {
                        match (&ev.token_type, &ev.taker_side) {
                            (crate::types::TokenType::Yes, crate::types::OrderSide::Buy) => {
                                diff.yes_bids_removed.push(ev.taker_order_id as i64);
                                diff.yes_bids_added.push(updated_taker)
                            }
                            (crate::types::TokenType::Yes, crate::types::OrderSide::Sell) => {
                                diff.yes_asks_removed.push(ev.taker_order_id as i64);
                                diff.yes_asks_added.push(updated_taker)
                            }
                            (crate::types::TokenType::No, crate::types::OrderSide::Buy) => {
                                diff.no_bids_removed.push(ev.taker_order_id as i64);
                                diff.no_bids_added.push(updated_taker)
                            }
                            (crate::types::TokenType::No, crate::types::OrderSide::Sell) => {
                                diff.no_asks_removed.push(ev.taker_order_id as i64);
                                diff.no_asks_added.push(updated_taker)
                            }
                        }
                    } else if updated_taker.status == OrderStatus::Filled {
                        match (&ev.token_type, &ev.taker_side) {
                            (crate::types::TokenType::Yes, crate::types::OrderSide::Buy) => {
                                diff.yes_bids_removed.push(ev.taker_order_id as i64)
                            }
                            (crate::types::TokenType::Yes, crate::types::OrderSide::Sell) => {
                                diff.yes_asks_removed.push(ev.taker_order_id as i64)
                            }
                            (crate::types::TokenType::No, crate::types::OrderSide::Buy) => {
                                diff.no_bids_removed.push(ev.taker_order_id as i64)
                            }
                            (crate::types::TokenType::No, crate::types::OrderSide::Sell) => {
                                diff.no_asks_removed.push(ev.taker_order_id as i64)
                            }
                        }
                    }
                }
            }

            publish_diff(&mut publisher, ev.market_id as i32, &diff)?;

            // Publish a TradeTick so backend WS clients can update live candles
            let tick = TradeTick {
                market_id: ev.market_id as i32,
                token_type: match &ev.token_type {
                    crate::types::TokenType::Yes => "yes".to_string(),
                    crate::types::TokenType::No  => "no".to_string(),
                },
                price:           ev.price as i64,
                quantity:        ev.quantity as i64,
                event_timestamp: ev.timestamp,
            };
            publish_tick(&mut publisher, ev.market_id as i32, &tick)?;
        }

        DISC_ORDER_CANCELLED => {
            let ev = OrderCancelled::deserialize(payload)?;
            log::info!(
                "OrderCancelled: market={} order={} sig={}",
                ev.market_id,
                ev.order_id,
                sig
            );

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

            let mut diff = OrderbookDiff::new(slot as u64, ev.market_id as i32);
            match (&ev.token_type, &ev.side) {
                (crate::types::TokenType::Yes, crate::types::OrderSide::Buy) => {
                    diff.yes_bids_removed.push(ev.order_id as i64)
                }
                (crate::types::TokenType::Yes, crate::types::OrderSide::Sell) => {
                    diff.yes_asks_removed.push(ev.order_id as i64)
                }
                (crate::types::TokenType::No, crate::types::OrderSide::Buy) => {
                    diff.no_bids_removed.push(ev.order_id as i64)
                }
                (crate::types::TokenType::No, crate::types::OrderSide::Sell) => {
                    diff.no_asks_removed.push(ev.order_id as i64)
                }
            }
            publish_diff(&mut publisher, ev.market_id as i32, &diff)?;
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
                ev.initial_quantity as i64,
                ev.filled_quantity as i64,
                ev.orders_matched as i64,
                ev.timestamp,
            )
            .await?;
            // Market orders are taker-only. Each fill already emits a DISC_ORDER_MATCHED
            // event that handles the maker-side diff, so nothing extra is needed here.
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
            log::info!(
                "WinningSideSet: market={} outcome={:?} sig={}",
                ev.market_id,
                ev.winning_outcome,
                sig
            );
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
            log::info!(
                "FundsClaimed: market={} user={} sig={}",
                ev.market_id,
                ev.user,
                sig
            );
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

// Indexer will publish events in the Pubsub
fn publish_diff(
    publisher: &mut redis::Connection,
    market_id: i32,
    diff: &OrderbookDiff,
) -> Result<()> {
    let msg = serde_json::to_string(diff)?;
    let channel = format!("orderbook:market:{}", market_id);
    publisher
        .publish::<String, String, ()>(channel.clone(), msg)
        .map_err(|e| anyhow::anyhow!("Failed to publish to {}: {}", channel, e))?;
    Ok(())
}

fn publish_tick(
    publisher: &mut redis::Connection,
    market_id: i32,
    tick: &TradeTick,
) -> Result<()> {
    let msg = serde_json::to_string(tick)?;
    let channel = format!("trades:market:{}", market_id);
    publisher
        .publish::<String, String, ()>(channel.clone(), msg)
        .map_err(|e| anyhow::anyhow!("Failed to publish tick to {}: {}", channel, e))?;
    Ok(())
}

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
