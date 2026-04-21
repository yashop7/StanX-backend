use axum::{
    Json,
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde_json::{json, Value};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use db::Db;
use serde::Deserialize;
use std::{env, sync::Arc};

use crate::event_handler::handle_event;

#[derive(Clone)]
pub struct WebhookState {
    pub db: Arc<Db>,
    pub auth_header: Option<String>,
}

// Helius raw webhook payload — array of raw Solana transactions
#[derive(Deserialize)]
struct HeliusRawTx {
    slot: Option<u64>,
    transaction: Option<RawTxInner>,
    meta: Option<TxMeta>,
}

#[derive(Deserialize)]
struct RawTxInner {
    signatures: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct TxMeta {
    err: Option<serde_json::Value>,
    #[serde(rename = "logMessages")]
    log_messages: Option<Vec<String>>,
}

pub async fn helius_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    // Verify auth header if configured
    if let Some(expected) = &state.auth_header {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != expected {
            log::warn!("Webhook: rejected request with invalid auth header");
            return StatusCode::UNAUTHORIZED;
        }
    }

    log::info!("Webhook: received POST ({} bytes)", body.len());

    let txs: Vec<HeliusRawTx> = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Webhook: failed to parse payload: {}\nraw body: {}", e, String::from_utf8_lossy(&body));
            return StatusCode::BAD_REQUEST;
        }
    };

    // Spawn processing in background so Helius gets 200 immediately
    // and does not retry due to slow DB writes.
    tokio::spawn(async move {
        log::info!("Webhook: processing {} transaction(s)", txs.len());

        for tx in txs {
            let meta = match tx.meta {
                Some(m) => m,
                None => {
                    log::warn!("Webhook: transaction has no meta, skipping");
                    continue;
                }
            };

            if meta.err.is_some() {
                log::debug!("Webhook: skipping failed transaction");
                continue;
            }

            let slot = tx.slot.unwrap_or(0);
            let sig = tx
                .transaction
                .as_ref()
                .and_then(|t| t.signatures.as_ref())
                .and_then(|s| s.first())
                .map(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();

            log::info!("Webhook: processing tx sig={} slot={}", sig, slot);

            let logs = match meta.log_messages {
                Some(l) => l,
                None => {
                    log::warn!("Webhook: tx sig={} has no logMessages — make sure webhook type is 'raw'", sig);
                    continue;
                }
            };

            let mut events_found = 0usize;
            for log_line in logs {
                if let Some(val) = log_line.strip_prefix("Program data: ") {
                    events_found += 1;
                    if let Ok(data) = B64.decode(val) {
                        log::debug!("Webhook: decoding event {} bytes for sig={}", data.len(), sig);
                        if let Err(e) = handle_event(&sig, slot, &data, &state.db).await {
                            log::error!("Webhook: event error sig={}: {}", sig, e);
                        }
                    } else {
                        log::warn!("Webhook: failed to base64-decode program data for sig={}", sig);
                    }
                }
            }

            if events_found == 0 {
                log::debug!("Webhook: no 'Program data:' entries in tx sig={}", sig);
            } else {
                log::info!("Webhook: processed {} event(s) for sig={}", events_found, sig);
            }
        }
    });

    // Return 200 immediately — Helius will not retry
    StatusCode::OK
}

async fn health() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub fn webhook_router(db: Arc<Db>) -> Router {
    let auth_header = env::var("HELIUS_AUTH_HEADER").ok();
    if auth_header.is_some() {
        log::info!("Webhook: auth header verification enabled");
    } else {
        log::warn!("Webhook: HELIUS_AUTH_HEADER not set — endpoint is unauthenticated");
    }

    let state = WebhookState { db, auth_header };
    Router::new()
        .route("/health", get(health))
        .route("/helius/webhook", post(helius_webhook))
        .with_state(state)
}
