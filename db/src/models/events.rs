use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::Db;

// ============================================================================
// Database Models for Blockchain Events
// ============================================================================

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbMarketInitialized {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub authority: String,
    pub settlement_deadline: i64,
    pub collateral_mint: String,
    pub outcome_yes_mint: String,
    pub outcome_no_mint: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbOrderPlaced {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub order_id: i64,
    pub user: String,
    pub side: String, // "Buy" or "Sell"
    pub token_type: String, // "Yes" or "No"
    pub price: i64,
    pub quantity: i64,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbOrderMatched {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub taker_order_id: i64,
    pub maker_order_id: i64,
    pub taker: String,
    pub maker: String,
    pub token_type: String, // "Yes" or "No"
    pub price: i64,
    pub quantity: i64,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbOrderCancelled {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub order_id: i64,
    pub user: String,
    pub side: String, // "Buy" or "Sell"
    pub token_type: String, // "Yes" or "No"
    pub remaining_quantity: i64,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbMarketOrderExecuted {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub user: String,
    pub side: String, // "Buy" or "Sell"
    pub token_type: String, // "Yes" or "No"
    pub total_quantity: i64,
    pub orders_matched: i64,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbTokensSplit {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub user: String,
    pub amount: i64,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbTokensMerged {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub user: String,
    pub amount: i64,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbWinningSideSet {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub winning_outcome: String, // "OutcomeA", "OutcomeB", or "Neither"
    pub authority: String,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbRewardsClaimed {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub user: String,
    pub collateral_amount: i64,
    pub yes_tokens_burned: i64,
    pub no_tokens_burned: i64,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbMarketClosed {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub authority: String,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbMetadataUpdated {
    pub id: i32,
    pub signature: String,
    pub slot: i64,
    pub market_id: i32,
    pub authority: String,
    pub new_metadata_url: String,
    pub timestamp: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// Database Functions for Storing Events
// ============================================================================

impl Db {
    /// Store a MarketInitialized event
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
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO market_initialized 
            (signature, slot, market_id, authority, settlement_deadline, 
             collateral_mint, outcome_yes_mint, outcome_no_mint)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot as i64,
            market_id,
            authority,
            settlement_deadline,
            collateral_mint,
            outcome_yes_mint,
            outcome_no_mint
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store an OrderPlaced event
    pub async fn store_order_placed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        order_id: i64,
        user: &str,
        side: &str,
        token_type: &str,
        price: i64,
        quantity: i64,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO order_placed 
            (signature, slot, market_id, order_id, user, side, token_type, 
             price, quantity, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (signature, order_id) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            order_id,
            user,
            side,
            token_type,
            price,
            quantity,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store an OrderMatched event
    pub async fn store_order_matched(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        taker_order_id: i64,
        maker_order_id: i64,
        taker: &str,
        maker: &str,
        token_type: &str,
        price: i64,
        quantity: i64,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO order_matched 
            (signature, slot, market_id, taker_order_id, maker_order_id, 
             taker, maker, token_type, price, quantity, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (signature, taker_order_id, maker_order_id) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            taker_order_id,
            maker_order_id,
            taker,
            maker,
            token_type,
            price,
            quantity,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store an OrderCancelled event
    pub async fn store_order_cancelled(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        order_id: i64,
        user: &str,
        side: &str,
        token_type: &str,
        remaining_quantity: i64,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO order_cancelled 
            (signature, slot, market_id, order_id, user, side, token_type, 
             remaining_quantity, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (signature, order_id) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            order_id,
            user,
            side,
            token_type,
            remaining_quantity,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store a MarketOrderExecuted event
    pub async fn store_market_order_executed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user: &str,
        side: &str,
        token_type: &str,
        total_quantity: i64,
        orders_matched: i64,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO market_order_executed 
            (signature, slot, market_id, user, side, token_type, 
             total_quantity, orders_matched, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            user,
            side,
            token_type,
            total_quantity,
            orders_matched,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store a TokensSplit event
    pub async fn store_tokens_split(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user: &str,
        amount: i64,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO tokens_split 
            (signature, slot, market_id, user, amount, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            user,
            amount,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store a TokensMerged event
    pub async fn store_tokens_merged(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user: &str,
        amount: i64,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO tokens_merged 
            (signature, slot, market_id, user, amount, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            user,
            amount,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store a WinningSideSet event
    pub async fn store_winning_side_set(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        winning_outcome: &str,
        authority: &str,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO winning_side_set 
            (signature, slot, market_id, winning_outcome, authority, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            winning_outcome,
            authority,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store a RewardsClaimed event
    pub async fn store_rewards_claimed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        user: &str,
        collateral_amount: i64,
        yes_tokens_burned: i64,
        no_tokens_burned: i64,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO rewards_claimed 
            (signature, slot, market_id, user, collateral_amount, 
             yes_tokens_burned, no_tokens_burned, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            user,
            collateral_amount,
            yes_tokens_burned,
            no_tokens_burned,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store a MarketClosed event
    pub async fn store_market_closed(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        authority: &str,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO market_closed 
            (signature, slot, market_id, authority, timestamp)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            authority,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    /// Store a MetadataUpdated event
    pub async fn store_metadata_updated(
        &self,
        signature: &str,
        slot: i64,
        market_id: i32,
        authority: &str,
        new_metadata_url: &str,
        timestamp: i64,
    ) -> Result<i32> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO metadata_updated 
            (signature, slot, market_id, authority, new_metadata_url, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signature,
            slot,
            market_id,
            authority,
            new_metadata_url,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(rec.map(|r| r.id).unwrap_or(-1))
    }

    // ========================================================================
    // Query Functions - Get events from database
    // ========================================================================

    /// Get all order placed events for a market
    pub async fn get_orders_by_market(&self, market_id: i32) -> Result<Vec<DbOrderPlaced>> {
        let orders = sqlx::query_as!(
            DbOrderPlaced,
            r#"
            SELECT * FROM order_placed 
            WHERE market_id = $1 
            ORDER BY timestamp DESC
            "#,
            market_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(orders)
    }

    /// Get all order matched events for a market
    pub async fn get_matches_by_market(&self, market_id: i32) -> Result<Vec<DbOrderMatched>> {
        let matches = sqlx::query_as!(
            DbOrderMatched,
            r#"
            SELECT * FROM order_matched 
            WHERE market_id = $1 
            ORDER BY timestamp DESC
            "#,
            market_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(matches)
    }

    /// Get all events for a specific user
    pub async fn get_user_orders(&self, user: &str) -> Result<Vec<DbOrderPlaced>> {
        let orders = sqlx::query_as!(
            DbOrderPlaced,
            r#"
            SELECT * FROM order_placed 
            WHERE user = $1 
            ORDER BY timestamp DESC
            "#,
            user
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(orders)
    }

    /// Get market initialization data
    pub async fn get_market_init(&self, market_id: i32) -> Result<Option<DbMarketInitialized>> {
        let market = sqlx::query_as!(
            DbMarketInitialized,
            r#"
            SELECT * FROM market_initialized 
            WHERE market_id = $1
            "#,
            market_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(market)
    }
}
