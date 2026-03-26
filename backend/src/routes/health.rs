use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, http::StatusCode, Json};
use redis::Commands;
use serde::Serialize;

use crate::state::state::AppState;

const STALE_AFTER_SECS: u64 = 30;

#[derive(Serialize)]
pub struct HealthResponse {
    pub indexer_ok: bool,
    /// Unix timestamp of the last heartbeat written by the indexer.
    pub last_heartbeat: Option<u64>,
    /// How many seconds ago that heartbeat was (for quick debugging).
    pub seconds_since_heartbeat: Option<u64>,
}

pub async fn health_handler(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let last_heartbeat: Option<u64> = (|| {
        let mut conn = state.redis.get_connection().ok()?;
        conn.get::<_, u64>("indexer:heartbeat").ok()
    })();

    let (indexer_ok, seconds_since) = match last_heartbeat {
        Some(ts) => {
            let diff = now.saturating_sub(ts);
            (diff <= STALE_AFTER_SECS, Some(diff))
        }
        None => (false, None),
    };

    let status = if indexer_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(HealthResponse {
            indexer_ok,
            last_heartbeat,
            seconds_since_heartbeat: seconds_since,
        }),
    )
}
