mod event_handler;
mod types;
mod webhook;

use std::{
    collections::HashSet,
    env,
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use db::Db;
use futures_util::StreamExt;
use redis::Commands;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::{RpcTransactionConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{option_serializer::OptionSerializer, UiTransactionEncoding};

use crate::event_handler::handle_event;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // Try local .env first, then walk up to workspace root
    dotenvy::dotenv().ok();
    dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env")).ok();

    let db_url = env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL not set in environment"))?;
    let program =
        env::var("PROGRAM_ID").map_err(|_| anyhow::anyhow!("PROGRAM_ID not set in environment"))?;
    let program_id =
        Pubkey::from_str(&program).map_err(|_| anyhow::anyhow!("Invalid PROGRAM_ID"))?;
    let rpc_url = env::var("SOLANA_WS_RPC_URL")
        .unwrap_or_else(|_| "wss://api.devnet.solana.com/".to_string());

    let redis_port =
        env::var("REDIS_PORT").map_err(|_| anyhow::anyhow!("REDIS_PORT not set in environment"))?;
    let redis_address = env::var("REDIS_ADDRESS")
        .map_err(|_| anyhow::anyhow!("REDIS_ADDRESS not set in environment"))?;
    let redis_password = env::var("REDIS_PASSWORD").unwrap_or_default();
    let redis_url = if redis_password.is_empty() {
        format!("redis://{}:{}", redis_address, redis_port)
    } else {
        format!("redis://default:{}@{}:{}", redis_password, redis_address, redis_port)
    };
    let http_url = env::var("SOLANA_HTTP_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    
    let db = Arc::new(Db::new(&db_url).await?);
    let rpc_client = RpcClient::new(http_url);

    let filter = RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]);
    let config = RpcTransactionLogsConfig { commitment: None };

    // Heartbeat: write current unix timestamp to Redis every 10s so the backend
    // can expose /health and the frontend knows the indexer is alive.
    let heartbeat_redis_url = redis_url.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(client) = redis::Client::open(heartbeat_redis_url.clone()) {
                if let Ok(mut conn) = client.get_connection() {
                    let ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let _: Result<(), _> = conn.set_ex("indexer:heartbeat", ts, 60);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });


    // Spawning Helius webhook HTTP server
    let webhook_db = db.clone();
    tokio::spawn(async move {
        let app = webhook::webhook_router(webhook_db);
        let webhook_port = env::var("WEBHOOK_PORT").unwrap_or_else(|_| "3004".to_string());
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", webhook_port))
            .await
            .expect("Failed to bind webhook port");
        log::info!("Webhook server listening on port {}", webhook_port);
        axum::serve(listener, app).await.expect("Webhook server failed");
    });

    if let Err(e) = backfill(&*db, &rpc_client, &program_id).await {
        log::error!("Backfill failed: {}", e);
    }

    // WS listener is disabled when HELIUS_WEBHOOK_ONLY=true to avoid
    // duplicate events (and the lock contention that comes with them)
    // when the Helius webhook is the primary event source.
    if env::var("HELIUS_WEBHOOK_ONLY").as_deref() == Ok("true") {
        log::info!("HELIUS_WEBHOOK_ONLY=true, WS listener disabled, running webhook-only mode");
        std::future::pending::<()>().await;
    }

    let mut backoff_secs: u64 = 1;

    loop {
        log::info!("Indexer: connecting to WS {}", rpc_url);

        let client = match PubsubClient::new(&rpc_url).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("WS connect failed: {}. Retrying in {}s", e, backoff_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60);
                continue;
            }
        };

        let (mut log_stream, _unsubscribe) =
            match client.logs_subscribe(filter.clone(), config.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Subscribe failed: {}. Retrying in {}s", e, backoff_secs);
                    tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(60);
                    continue;
                }
            };

        log::info!("Indexer listening for program {}", program_id);
        backoff_secs = 1; // reset after successful connect

        let mut seen_signatures: HashSet<String> = HashSet::new();

        while let Some(msg) = log_stream.next().await {
            if msg.value.err.is_some() {
                continue;
            }

            let slot = msg.context.slot;
            let signature = &msg.value.signature;

            // Skip null signature (simulated/preflight transactions)
            if signature.chars().all(|c| c == '1') {
                log::debug!("Skipping simulated tx sig={}", signature);
                continue;
            }

            if seen_signatures.contains(signature) {
                log::debug!("Skipping duplicate sig={}", signature);
                continue;
            }
            seen_signatures.insert(signature.clone());

            for log in msg.value.logs {
                if let Some(val) = log.strip_prefix("Program data: ") {
                    // Data is Base 64 Encoded
                    if let Ok(data) = B64.decode(val) {
                        if let Err(e) = event_handler::handle_event(signature, slot, &data, &*db).await {
                            log::error!(
                                "Event handling error sig={}: {} | raw_hex={}",
                                signature,
                                e,
                                data.iter()
                                    .map(|b| format!("{:02x}", b))
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            );
                        }
                    }
                }
            }
        }

        // Stream ended (WS disconnected) — reconnect immediately
        log::warn!("Indexer: log stream ended, reconnecting...");
    }
}

// Backfill the function
// Run on Startup
// See the last Indexed Cursor and See the Events
pub async fn backfill(db: &Db, rpc_client: &RpcClient, program_id: &Pubkey) -> Result<()> {
    // Let's read from the last Indexed Cursor
    let until_signature;

    if let Ok(Some((sig, _slot))) = db.get_cursor().await {
        match sig.parse::<Signature>() {
            Ok(sig) => {
                until_signature = sig;
            }
            Err(_e) => {
                log::warn!("Signature Invalid");
                return Ok(());
            }
        }
    } else {
        log::info!("Backfill: No cursor, we are making a fresh start, So skipping Backfill");
        return Ok(());
    }

    log::info!(
        "Backfill: fetching missed transactions after {:?}",
        until_signature.clone()
    );

    let mut all_sig = Vec::new();
    let mut before: Option<Signature> = None;

    loop {
        // Now we will Start
        // From https://solana.com/docs/rpc/http/getsignaturesforaddress
        // before : Start searching backwards from this transaction signature. If not provided the search starts from the top of the highest max confirmed block.
        // limit : Search until this transaction signature, if found before limit reached

        let config = GetConfirmedSignaturesForAddress2Config {
            before,
            until: Some(until_signature),
            limit: Some(1000),
            commitment: None,
        };

        // ordered from newest to oldest transaction
        let sig_list: Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature> =
            rpc_client
                .get_signatures_for_address_with_config(program_id, config)
                .await?;
        let sign_list_len = sig_list.len();
        let oldest = sig_list.last().and_then(|s| s.signature.parse().ok());

        all_sig.extend(sig_list);

        if sign_list_len < 1000 {
            break;
        }

        before = oldest;
    }

    if all_sig.is_empty() {
        log::info!("Backfill: already up to date");
        return Ok(());
    }

    for sig_info in all_sig.iter().rev() {
        if sig_info.err.is_some() {
            continue;
        }

        let sig_str = &sig_info.signature.parse::<Signature>()?;
        // Fetch full transaction from HTTP RPC
        let tx = match rpc_client
            .get_transaction_with_config(
                sig_str,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: None,
                    max_supported_transaction_version: Some(0),
                },
            )
            .await
        {
            Ok(tx) => tx,
            Err(e) => {
                log::error!("Backfill: failed to fetch tx {}: {}", sig_str, e);
                continue;
            }
        };

        let slot = tx.slot;
        let meta = match tx.transaction.meta {
            Some(m) => m,
            None => continue,
        };

        if meta.err.is_some() {
            continue;
        }

        let logs = match meta.log_messages {
            OptionSerializer::Some(logs) => logs,
            _ => continue,
        };

        for log_line in logs {
            if let Some(val) = log_line.strip_prefix("Program data: ") {
                if let Ok(data) = B64.decode(val) {
                    if let Err(e) = handle_event(&sig_info.signature, slot, &data, db).await {
                        log::error!(
                            "Backfill event error sig={}: {} | raw_hex={}",
                            sig_str,
                            e,
                            data.iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<_>>()
                                .join(" ")
                        );
                    }
                }
            }
        }
    }

    log::info!("Backfill: complete");

    Ok(())
}
