use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use db::models::events::{Market, OrderbookResponse, HistoryPoint, Trade, LiveOrder};
use serde::{Deserialize, Serialize};

use crate::state::state::AppState;

#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct HistoryParams {
    pub token: Option<String>,
    pub period: Option<String>,
}

#[derive(Serialize)]
pub struct PriceHistoryResponse {
    pub history: Vec<HistoryPoint>,
}

const VALID_PERIODS: &[&str] = &["1H", "6H", "1D", "1W", "1M", "3M", "ALL"];

/// GET /markets
pub async fn get_markets(
    State(state): State<AppState>,
) -> Result<Json<Vec<Market>>, (StatusCode, String)> {
    let markets = state
        .db
        .get_active_markets()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(markets))
}

/// GET /markets/:market_id
pub async fn get_market(
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
) -> Result<Json<Market>, (StatusCode, String)> {
    let market = state
        .db
        .get_market(market_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Market not found".to_string()))?;
    Ok(Json(market))
}

/// GET /markets/:market_id/orderbook
pub async fn get_orderbook(
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
) -> Result<Json<OrderbookResponse>, (StatusCode, String)> {
    let book = state
        .db
        .get_orderbook(market_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(book))
}

/// GET /markets/:market_id/trades?limit=50
pub async fn get_trades(
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<Trade>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(200);
    let trades = state
        .db
        .get_trades(market_id, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(trades))
}

/// GET /markets/:market_id/orders/:user_pubkey
pub async fn get_user_orders(
    State(state): State<AppState>,
    Path((market_id, user_pubkey)): Path<(i32, String)>,
) -> Result<Json<Vec<LiveOrder>>, (StatusCode, String)> {
    let orders = state
        .db
        .get_user_orders(&user_pubkey, market_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(orders))
}

/// GET /markets/:market_id/prices?token=yes&period=1D
/// Time-bucketed price history for the line chart.
/// Response: `{ "history": [{ "t": 1711929600000, "p": "6500" }, ...] }`
pub async fn get_prices(
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<PriceHistoryResponse>, (StatusCode, String)> {
    let token_raw = params.token.as_deref().unwrap_or("yes");
    let token = match token_raw {
        "yes" | "Yes" => "Yes",
        "no" | "No" => "No",
        _ => return Err((StatusCode::BAD_REQUEST, "token must be 'yes' or 'no'".into())),
    };
    let period = params.period.as_deref().unwrap_or("1D");
    if !VALID_PERIODS.contains(&period) {
        return Err((StatusCode::BAD_REQUEST, format!("period must be one of: {}", VALID_PERIODS.join(", "))));
    }
    let history = state
        .db
        .get_price_history(market_id, token, period)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(PriceHistoryResponse { history }))
}

/// GET /user/:user_pubkey/markets
pub async fn get_user_markets(
    State(state): State<AppState>,
    Path(user_pubkey): Path<String>,
) -> Result<Json<Vec<Market>>, (StatusCode, String)> {
    let markets = state
        .db
        .get_user_markets(&user_pubkey)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(markets))
}

/// GET /user/:user_pubkey/trades?limit=50
pub async fn get_user_trades(
    State(state): State<AppState>,
    Path(user_pubkey): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<Trade>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(200);
    let trades = state
        .db
        .get_user_trades(&user_pubkey, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(trades))
}
