use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::Db;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "order_side", rename_all = "PascalCase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "token_type", rename_all = "PascalCase")]
pub enum TokenType {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "winning_outcome")]
pub enum WinningOutcome {
    OutcomeA,
    OutcomeB,
    Neither,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "order_status", rename_all = "snake_case")]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "market_status", rename_all = "snake_case")]
pub enum MarketStatus {
    Active,
    Settled,
    Closed,
}

// =============================================================================
// Materialized state models
// =============================================================================

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Market {
    pub market_id: i32,
    pub authority: String,
    pub settlement_deadline: i64,
    pub collateral_mint: String,
    pub outcome_yes_mint: String,
    pub outcome_no_mint: String,
    pub meta_data_url: String,
    pub status: MarketStatus,
    pub winning_outcome: Option<WinningOutcome>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LiveOrder {
    pub order_id: i64,
    pub market_id: i32,
    pub user_pubkey: String,
    pub side: OrderSide,
    pub token_type: TokenType,
    pub price: i64,
    pub original_quantity: i64,
    pub remaining_quantity: i64,
    pub status: OrderStatus,
    pub placed_at: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Trade {
    pub id: i32,
    pub signature: String,
    pub market_id: i32,
    pub maker_order_id: i64,
    pub taker_side: OrderSide,
    pub taker: String,
    pub maker: String,
    pub token_type: TokenType,
    pub price: i64,
    pub quantity: i64,
    pub event_timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// HistoryPoint one point on the probability line chart
// Follows format: { t: unix_ms, p: "0.65" }
// p is the raw on-chain price as a string — frontend divides by price_decimals

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct HistoryPoint {
    /// Unix timestamp in milliseconds
    pub t: i64,
    /// Last traded price in this bucket (raw on-chain integer, as string)
    pub p: String,
}

/// Oracle-computed resolution result for a market.
/// Stored by oracle-service after the deadline passes.
/// The market creator reads this and builds the SetWinner tx in their wallet.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketResolution {
    pub market_id:    i32,
    pub outcome:      String, // "OutcomeA","OutcomeB"
    pub actual_value: i64,
    pub threshold:    i64,
    pub metric:       String,
    pub video_id:     String,
    pub resolved_at:  chrono::DateTime<chrono::Utc>,
}

// Orderbook response shape — what the API returns
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderbookResponse {
    pub market_id: i32,
    pub yes_buy_orders: Vec<LiveOrder>,
    pub yes_sell_orders: Vec<LiveOrder>,
    pub no_buy_orders: Vec<LiveOrder>,
    pub no_sell_orders: Vec<LiveOrder>,
}

// Database write operations — called by event-listener
impl Db {
    pub async fn store_market_initialized(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        authority: &str,
        settlement_deadline: i64,
        collateral_mint: &str,
        outcome_yes_mint: &str,
        outcome_no_mint: &str,
        meta_data_url: &str,
        event_timestamp: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO event_market_initialized
               (signature, slot, market_id, authority, settlement_deadline,
                collateral_mint, outcome_yes_mint, outcome_no_mint, meta_data_url, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
               ON CONFLICT (signature, market_id) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(authority)
        .bind(settlement_deadline)
        .bind(collateral_mint)
        .bind(outcome_yes_mint)
        .bind(outcome_no_mint)
        .bind(meta_data_url)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO markets
               (market_id, authority, settlement_deadline, collateral_mint,
                outcome_yes_mint, outcome_no_mint, meta_data_url)
               VALUES ($1,$2,$3,$4,$5,$6,$7)
               ON CONFLICT (market_id) DO NOTHING"#,
        )
        .bind(market_id)
        .bind(authority)
        .bind(settlement_deadline)
        .bind(collateral_mint)
        .bind(outcome_yes_mint)
        .bind(outcome_no_mint)
        .bind(meta_data_url)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_market_closed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        authority: &str,
        event_timestamp: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO event_market_closed
               (signature, slot, market_id, authority, event_timestamp)
               VALUES ($1,$2,$3,$4,$5)
               ON CONFLICT (signature, market_id) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(authority)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE markets SET status = 'closed', updated_at = NOW() WHERE market_id = $1",
        )
        .bind(market_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_winning_side_set(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        winning_outcome: WinningOutcome,
        authority: &str,
        event_timestamp: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO event_winning_side_set
               (signature, slot, market_id, winning_outcome, authority, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6)
               ON CONFLICT (signature, market_id) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(winning_outcome)
        .bind(authority)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE markets SET status = 'settled', winning_outcome = $2, updated_at = NOW() WHERE market_id = $1",
        )
        .bind(market_id)
        .bind(winning_outcome)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_metadata_updated(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        authority: &str,
        new_metadata_url: &str,
        event_timestamp: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO event_metadata_updated
               (signature, slot, market_id, authority, new_metadata_url, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6)
               ON CONFLICT (signature, market_id) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(authority)
        .bind(new_metadata_url)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE markets SET meta_data_url = $2, updated_at = NOW() WHERE market_id = $1",
        )
        .bind(market_id)
        .bind(new_metadata_url)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_order_placed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        order_id: i64,
        user_pubkey: &str,
        side: OrderSide,
        token_type: TokenType,
        price: i64,
        quantity: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO event_order_placed
               (signature, slot, market_id, order_id, user_pubkey, side, token_type, price, quantity, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
               ON CONFLICT (signature, order_id) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(order_id)
        .bind(user_pubkey)
        .bind(side)
        .bind(token_type)
        .bind(price)
        .bind(quantity)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        // Insert into live_orders (materialized orderbook)
        sqlx::query(
            r#"INSERT INTO live_orders
               (order_id, market_id, user_pubkey, side, token_type, price,
                original_quantity, remaining_quantity, status, placed_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$7,'open',$8)
               ON CONFLICT (market_id, order_id) DO NOTHING"#,
        )
        .bind(order_id)
        .bind(market_id)
        .bind(user_pubkey)
        .bind(side)
        .bind(token_type)
        .bind(price)
        .bind(quantity)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_order_matched(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        maker_order_id: i64,
        taker_order_id: i64, // 0 = market order (no live_order to update)
        taker_side: OrderSide,
        taker: &str,
        maker: &str,
        token_type: TokenType,
        price: i64,
        quantity: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // 1. Event log
        sqlx::query(
            r#"INSERT INTO event_order_matched
               (signature, slot, market_id, maker_order_id, taker_side, taker, maker,
                token_type, price, quantity, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
               ON CONFLICT (signature, maker_order_id) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(maker_order_id)
        .bind(taker_side)
        .bind(taker)
        .bind(maker)
        .bind(token_type)
        .bind(price)
        .bind(quantity)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        // 2. Trade history
        sqlx::query(
            r#"INSERT INTO trades
               (signature, market_id, maker_order_id, taker_side, taker, maker,
                token_type, price, quantity, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
        )
        .bind(signature)
        .bind(market_id)
        .bind(maker_order_id)
        .bind(taker_side)
        .bind(taker)
        .bind(maker)
        .bind(token_type)
        .bind(price)
        .bind(quantity)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        // 3. Update maker's live order — reduce remaining quantity
        sqlx::query(
            r#"UPDATE live_orders
               SET remaining_quantity = remaining_quantity - $3,
                   status = CASE
                       WHEN remaining_quantity - $3 <= 0 THEN 'filled'::order_status
                       ELSE 'partially_filled'::order_status
                   END,
                   updated_at = NOW()
               WHERE market_id = $1 AND order_id = $2"#,
        )
        .bind(market_id)
        .bind(maker_order_id)
        .bind(quantity)
        .execute(&mut *tx)
        .await?;

        // 4. Update taker's live order — only for limit orders (taker_order_id != 0)
        // Market orders (taker_order_id == 0) never rest on the book, nothing to update.
        if taker_order_id != 0 {
            sqlx::query(
                r#"UPDATE live_orders
                   SET remaining_quantity = remaining_quantity - $3,
                       status = CASE
                           WHEN remaining_quantity - $3 <= 0 THEN 'filled'::order_status
                           ELSE 'partially_filled'::order_status
                       END,
                       updated_at = NOW()
                   WHERE market_id = $1 AND order_id = $2"#,
            )
            .bind(market_id)
            .bind(taker_order_id)
            .bind(quantity)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_order_cancelled(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        order_id: i64,
        user_pubkey: &str,
        side: OrderSide,
        token_type: TokenType,
        remaining_quantity: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO event_order_cancelled
               (signature, slot, market_id, order_id, user_pubkey, side, token_type,
                remaining_quantity, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
               ON CONFLICT (signature, order_id) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(order_id)
        .bind(user_pubkey)
        .bind(side)
        .bind(token_type)
        .bind(remaining_quantity)
        .bind(event_timestamp)
        .execute(&mut *tx)
        .await?;

        // Mark order as cancelled in live_orders
        sqlx::query(
            r#"UPDATE live_orders
               SET status = 'cancelled', updated_at = NOW()
               WHERE market_id = $1 AND order_id = $2"#,
        )
        .bind(market_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_market_order_executed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user_pubkey: &str,
        side: OrderSide,
        token_type: TokenType,
        initial_quantity: i64,
        filled_quantity: i64,
        orders_matched: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        // Market orders are taker-only — the maker side updates happen via OrderMatched events.
        sqlx::query(
            r#"INSERT INTO event_market_order_executed
               (signature, slot, market_id, user_pubkey, side, token_type,
                initial_quantity, filled_quantity, orders_matched, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
               ON CONFLICT (signature) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(user_pubkey)
        .bind(side)
        .bind(token_type)
        .bind(initial_quantity)
        .bind(filled_quantity)
        .bind(orders_matched)
        .bind(event_timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn store_tokens_split(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user_pubkey: &str,
        amount: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO event_tokens_split
               (signature, slot, market_id, user_pubkey, amount, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6)
               ON CONFLICT (signature) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(user_pubkey)
        .bind(amount)
        .bind(event_timestamp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn store_tokens_merged(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user_pubkey: &str,
        amount: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO event_tokens_merged
               (signature, slot, market_id, user_pubkey, amount, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6)
               ON CONFLICT (signature) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(user_pubkey)
        .bind(amount)
        .bind(event_timestamp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Claim events ─────────────────────────────────────────────────────

    pub async fn store_rewards_claimed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user_pubkey: &str,
        collateral_amount: i64,
        yes_tokens_burned: i64,
        no_tokens_burned: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO event_rewards_claimed
               (signature, slot, market_id, user_pubkey, collateral_amount,
                yes_tokens_burned, no_tokens_burned, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
               ON CONFLICT (signature) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(user_pubkey)
        .bind(collateral_amount)
        .bind(yes_tokens_burned)
        .bind(no_tokens_burned)
        .bind(event_timestamp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn store_funds_claimed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user_pubkey: &str,
        collateral_amount: i64,
        yes_amount: i64,
        no_amount: i64,
        event_timestamp: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO event_funds_claimed
               (signature, slot, market_id, user_pubkey, collateral_amount,
                yes_amount, no_amount, event_timestamp)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
               ON CONFLICT (signature) DO NOTHING"#,
        )
        .bind(signature)
        .bind(slot)
        .bind(market_id)
        .bind(user_pubkey)
        .bind(collateral_amount)
        .bind(yes_amount)
        .bind(no_amount)
        .bind(event_timestamp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Indexer cursor ───────────────────────────────────────────────────

    pub async fn update_cursor(&self, signature: &str, slot: i64) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO indexer_cursor (id, last_signature, last_slot)
               VALUES (1, $1, $2)
               ON CONFLICT (id)
               DO UPDATE SET last_signature = $1, last_slot = $2, updated_at = NOW()"#,
        )
        .bind(signature)
        .bind(slot)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_cursor(&self) -> Result<Option<(String, i64)>> {
        let row: Option<(String, i64)> =
            sqlx::query_as("SELECT last_signature, last_slot FROM indexer_cursor WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    // KEEPING THEM ON HOLD, WE WILL FETCH THEM FROM THE IN-MEMORY ORDERBOOK

    // READ QUERIES — called by the backend API
    /// Get the full orderbook for a market (only open/partially_filled orders)
    pub async fn get_orderbook(&self, market_id: i32) -> Result<OrderbookResponse> {
        let orders: Vec<LiveOrder> = sqlx::query_as(
            r#"SELECT order_id, market_id, user_pubkey, side, token_type, price,
                      original_quantity, remaining_quantity, status, placed_at, updated_at
               FROM live_orders
               WHERE market_id = $1 AND status IN ('open', 'partially_filled')
               ORDER BY price"#,
        )
        .bind(market_id)
        .fetch_all(&self.pool)
        .await?;

        let mut book = OrderbookResponse {
            market_id,
            yes_buy_orders: Vec::new(),
            yes_sell_orders: Vec::new(),
            no_buy_orders: Vec::new(),
            no_sell_orders: Vec::new(),
        };

        for order in orders {
            match (order.token_type, order.side) {
                (TokenType::Yes, OrderSide::Buy) => book.yes_buy_orders.push(order),
                (TokenType::Yes, OrderSide::Sell) => book.yes_sell_orders.push(order),
                (TokenType::No, OrderSide::Buy) => book.no_buy_orders.push(order),
                (TokenType::No, OrderSide::Sell) => book.no_sell_orders.push(order),
            }
        }

        // Buy orders: highest price first; Sell orders: lowest price first
        book.yes_buy_orders.sort_by(|a, b| b.price.cmp(&a.price));
        book.yes_sell_orders.sort_by(|a, b| a.price.cmp(&b.price));
        book.no_buy_orders.sort_by(|a, b| b.price.cmp(&a.price));
        book.no_sell_orders.sort_by(|a, b| a.price.cmp(&b.price));

        Ok(book)
    }

    /// Get a single live order by market + order id (used after a match to read updated remaining_quantity)
    pub async fn get_live_order(&self, market_id: i32, order_id: i64) -> Result<Option<LiveOrder>> {
        let order = sqlx::query_as(
            r#"SELECT order_id, market_id, user_pubkey, side, token_type, price,
                      original_quantity, remaining_quantity, status, placed_at, updated_at
               FROM live_orders WHERE market_id = $1 AND order_id = $2"#,
        )
        .bind(market_id)
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(order)
    }

    /// Get a single market by ID
    pub async fn get_market(&self, market_id: i32) -> Result<Option<Market>> {
        println!("market_id: {}", market_id.clone());
        let market = sqlx::query_as("SELECT * FROM markets WHERE market_id = $1")
            .bind(market_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(market)
    }

    /// Get all active markets
    pub async fn get_active_markets(&self) -> Result<Vec<Market>> {
        let markets = sqlx::query_as(
            "SELECT * FROM markets WHERE status = 'active' ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(markets)
    }

    /// Get markets where the settlement deadline has passed but not yet settled.
    /// Used by the oracle-service cron to find markets that need resolution.
    pub async fn get_unsettled_expired_markets(&self, now_unix: i64) -> Result<Vec<Market>> {
        let markets = sqlx::query_as(
            r#"SELECT * FROM markets
               WHERE status = 'active'
                 AND settlement_deadline <= $1
               ORDER BY settlement_deadline ASC"#,
        )
        .bind(now_unix)
        .fetch_all(&self.pool)
        .await?;
        Ok(markets)
    }

    /// Store the oracle-computed resolution for a market.
    pub async fn store_resolution(
        &self,
        market_id: i32,
        outcome: &str,
        actual_value: i64,
        threshold: i64,
        metric: &str,
        video_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO market_resolutions
               (market_id, outcome, actual_value, threshold, metric, video_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (market_id) DO NOTHING"#,
        )
        .bind(market_id)
        .bind(outcome)
        .bind(actual_value)
        .bind(threshold)
        .bind(metric)
        .bind(video_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Fetch the oracle resolution for a market, if it exists.
    pub async fn get_resolution(&self, market_id: i32) -> Result<Option<MarketResolution>> {
        let row = sqlx::query_as(
            "SELECT * FROM market_resolutions WHERE market_id = $1",
        )
        .bind(market_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Get recent trades for a market
    pub async fn get_trades(&self, market_id: i32, limit: i64) -> Result<Vec<Trade>> {
        let trades = sqlx::query_as(
            r#"SELECT * FROM trades
               WHERE market_id = $1
               ORDER BY event_timestamp DESC
               LIMIT $2"#,
        )
        .bind(market_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(trades)
    }

    /// Get a user's open orders across a market
    pub async fn get_user_orders(
        &self,
        user_pubkey: &str,
        market_id: i32,
    ) -> Result<Vec<LiveOrder>> {
        let orders = sqlx::query_as(
            r#"SELECT * FROM live_orders
               WHERE user_pubkey = $1 AND market_id = $2
               ORDER BY placed_at DESC"#,
        )
        .bind(user_pubkey)
        .bind(market_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }

    /// Get a user's trade history
    pub async fn get_user_trades(&self, user_pubkey: &str, limit: i64) -> Result<Vec<Trade>> {
        let trades = sqlx::query_as(
            r#"SELECT * FROM trades
               WHERE taker = $1 OR maker = $1
               ORDER BY event_timestamp DESC
               LIMIT $2"#,
        )
        .bind(user_pubkey)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(trades)
    }

    /// Get time-bucketed price history for the line chart.
    /// Returns `[{ t: unix_ms, p: "price" }]` ordered oldest -> newest.
    ///
    /// `period` must be one of: 1H, 6H, 1D, 1W, 1M, 3M, ALL.
    /// The server downsamples by picking the last traded price in each time bucket.
    pub async fn get_price_history(
        &self,
        market_id: i32,
        token_type: &str,
        period: &str,
    ) -> Result<Vec<HistoryPoint>> {
        let (since_secs, bucket_secs): (i64, i64) = match period {
            "1H" => (3_600, 60),
            "6H" => (21_600, 300),
            "1D" => (86_400, 900),
            "1W" => (604_800, 3_600),
            "1M" => (2_592_000, 14_400),
            "3M" => (7_776_000, 43_200),
            "ALL" => (0, 86_400),
            _ => (86_400, 900), // default to 1D
        };

        let now_secs = chrono::Utc::now().timestamp();
        let since_ts = if since_secs == 0 {
            0
        } else {
            now_secs - since_secs
        };

        let points: Vec<HistoryPoint> = sqlx::query_as(
            r#"SELECT
                   (bucket * $3) * 1000       AS t,
                   last_price::TEXT            AS p
               FROM (
                   SELECT
                       event_timestamp / $3    AS bucket,
                       price                   AS last_price,
                       ROW_NUMBER() OVER (
                           PARTITION BY event_timestamp / $3
                           ORDER BY event_timestamp DESC
                       ) AS rn
                   FROM event_order_matched
                   WHERE market_id = $1
                     AND token_type = $2::token_type
                     AND event_timestamp >= $4
               ) sub
               WHERE rn = 1
               ORDER BY bucket ASC"#,
        )
        .bind(market_id)
        .bind(token_type)
        .bind(bucket_secs)
        .bind(since_ts)
        .fetch_all(&self.pool)
        .await?;
        Ok(points)
    }

    /// Get all markets created by a specific authority (user)
    pub async fn get_user_markets(&self, authority: &str) -> Result<Vec<Market>> {
        let markets =
            sqlx::query_as("SELECT * FROM markets WHERE authority = $1 ORDER BY created_at DESC")
                .bind(authority)
                .fetch_all(&self.pool)
                .await?;
        Ok(markets)
    }
}
