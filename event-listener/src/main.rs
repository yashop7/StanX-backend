mod event_handler;
mod types;

use std::{collections::HashSet, env, str::FromStr};

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use db::Db;
use futures_util::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();
    // Try local .env first, then walk up to workspace root
    dotenvy::dotenv().ok();
    dotenvy::from_path(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env")
    ).ok();

    let db_url = env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL not set in environment"))?;
    let program = env::var("PROGRAM_ID")
        .map_err(|_| anyhow::anyhow!("PROGRAM_ID not set in environment"))?;
    let program_id =
        Pubkey::from_str(&program).map_err(|_| anyhow::anyhow!("Invalid PROGRAM_ID"))?;
    let rpc_url = env::var("SOLANA_WS_RPC_URL")
        .unwrap_or_else(|_| "wss://api.devnet.solana.com/".to_string());
    


    let db = Db::new(&db_url).await?;
    let client = PubsubClient::new(&rpc_url).await?;

    let filter = RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]);
    let config = RpcTransactionLogsConfig { commitment: None };
    let (mut log_stream, _unsubscribe) = client.logs_subscribe(filter, config).await?;

    log::info!("Indexer listening for program {}", program_id);

    let mut seen_signatures: HashSet<String> = HashSet::new();

    while let Some(msg) = log_stream.next().await {

        if msg.value.err.is_some() {
            // Skipping the message with err
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
                    if let Err(e) =
                        event_handler::handle_event(signature, slot, &data, &db).await
                    {
                        log::error!(
                            "Event handling error sig={}: {} | raw_hex={}",
                            signature,
                            e,
                            data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
                        );
                    }
                }
            }
        }
    }
    Ok(())
}




    // while let Some(msg) = log_stream.next().await {
    //     // Skip failed transactions
    //     if msg.value.err.is_some() {
    //         continue;
    //     }

    //     let slot = msg.context.slot as i64;
    //     let signature = &msg.value.signature;

    //     for log in &msg.value.logs {
    //         if let Some(stripped) = log.strip_prefix("Program data: ") {
    //             if let Ok(data) = B64.decode(stripped) {
    //                 if let Err(e) =
    //                     event_handler::handle_event(signature, slot, &data, &db).await
    //                 {
    //                     log::error!("Event handling error sig={}: {}", signature, e);
    //                 }
    //             }
    //         }
    //     }
    // }
