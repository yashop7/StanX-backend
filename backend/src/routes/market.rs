use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use db::models::events::{Market, MarketResolution, OrderbookResponse, HistoryPoint, Trade, LiveOrder};
use serde::{Deserialize, Serialize};
use std::env;

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

// YouTube video preview, fetches metadata for market creation UI

#[derive(Deserialize)]
pub struct PreviewRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct PreviewResponse {
    pub video_id: String,
    pub title: String,
    pub thumbnail: String,
    pub channel_name: String,
    pub current_views: u64,
    pub current_likes: u64,
    pub current_comments: u64,
    pub published_at: String,
}

/// GET /markets/:market_id/resolution
/// Returns the oracle-computed outcome once the deadline has passed.
/// Frontend polls this. When it appears, show "Settle Market" button to the creator.
pub async fn get_resolution(
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
) -> Result<Json<MarketResolution>, (StatusCode, String)> {
    let resolution = state
        .db
        .get_resolution(market_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Resolution not yet available".to_string()))?;
    Ok(Json(resolution))
}

/// POST /markets/preview
/// Frontend sends { "url": "https://youtube.com/watch?v=abc123" }
/// Backend calls YouTube Data API and returns video metadata
pub async fn preview_video(
    Json(body): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, (StatusCode, String)> {
    let video_id = extract_video_id(&body.url)
        .ok_or((StatusCode::BAD_REQUEST, "Invalid YouTube URL".to_string()))?;

    let api_key = env::var("YOUTUBE_API_KEY")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "YouTube API key not configured".to_string()))?;

    let yt_url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=snippet,statistics&id={}&key={}",
        video_id, api_key
    );

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(&yt_url)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("YouTube API request failed: {}", e)))?
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("YouTube API parse failed: {}", e)))?;

    let items = resp["items"]
        .as_array()
        .ok_or((StatusCode::NOT_FOUND, "Video not found".to_string()))?;

    if items.is_empty() {
        return Err((StatusCode::NOT_FOUND, "Video not found".to_string()));
    }

    let item = &items[0];
    let snippet = &item["snippet"];
    let stats = &item["statistics"];

    let thumbnails = &snippet["thumbnails"];
    let thumbnail = thumbnails["maxres"]["url"]
        .as_str()
        .or_else(|| thumbnails["high"]["url"].as_str())
        .or_else(|| thumbnails["medium"]["url"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(Json(PreviewResponse {
        video_id: video_id.to_string(),
        title: snippet["title"].as_str().unwrap_or("").to_string(),
        thumbnail,
        channel_name: snippet["channelTitle"].as_str().unwrap_or("").to_string(),
        current_views: stats["viewCount"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        current_likes: stats["likeCount"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        current_comments: stats["commentCount"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        published_at: snippet["publishedAt"].as_str().unwrap_or("").to_string(),
    }))
}

/// Extracts video ID from various YouTube URL formats (Reels or normal Y.T Video)
fn extract_video_id(url: &str) -> Option<&str> {
    // https://www.youtube.com/watch?v=VIDEO_ID
    if let Some(pos) = url.find("v=") {
        let start = pos + 2;
        let rest = &url[start..];
        let end = rest.find('&').or_else(|| rest.find('#')).unwrap_or(rest.len());
        if end >= 11 {
            return Some(&rest[..end]);
        }
    }
    // https://youtu.be/VIDEO_ID
    if url.contains("youtu.be/") {
        if let Some(pos) = url.find("youtu.be/") {
            let start = pos + 9;
            let rest = &url[start..];
            let end = rest.find('?').or_else(|| rest.find('#')).unwrap_or(rest.len());
            if end >= 11 {
                return Some(&rest[..end]);
            }
        }
    }
    // https://www.youtube.com/shorts/VIDEO_ID
    if url.contains("/shorts/") {
        if let Some(pos) = url.find("/shorts/") {
            let start = pos + 8;
            let rest = &url[start..];
            let end = rest.find('?').or_else(|| rest.find('#')).unwrap_or(rest.len());
            if end >= 11 {
                return Some(&rest[..end]);
            }
        }
    }
    None
}
