mod event_handler;
mod types;

use anyhow::{Context, Result};
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::str::FromStr;
use std::env;

use event_handler::{parse_event, print_event, PredictionMarketEvent};

// Your program ID from the generated SDK
const PREDICTION_MARKET_PROGRAM_ID: &str = "G25hDisDca352CVMbrF49nZUGUiuJFBrAitfF7TTTHJc";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    log::info!("Starting Prediction Market Event Listener...");
    
    // Get RPC URL from environment or use default
    let rpc_ws_url = env::var("SOLANA_WS_URL")
        .unwrap_or_else(|_| "ws://localhost:8900".to_string());
    
    log::info!("Connecting to: {}", rpc_ws_url);
    log::info!("Monitoring program: {}", PREDICTION_MARKET_PROGRAM_ID);
    
    // Parse the program ID
    let program_id = Pubkey::from_str(PREDICTION_MARKET_PROGRAM_ID)
        .context("Failed to parse program ID")?;
    
    // Connect to Solana WebSocket
    let pubsub_client = PubsubClient::new(&rpc_ws_url)
        .await
        .context("Failed to connect to Solana WebSocket")?;
    
    log::info!("✓ Connected to Solana!");
    log::info!("Listening for events from program: {}", program_id);
    
    // Subscribe to logs for our program
    let config = RpcTransactionLogsConfig {
        commitment: Some(CommitmentConfig::confirmed()),
    };
    
    let (mut logs_subscription, _unsubscribe) = pubsub_client
        .logs_subscribe(
            RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]),
            config,
        )
        .await
        .context("Failed to subscribe to logs")?;
    
    log::info!("✓ Subscribed to program logs!");
    println!("\n🎯 Event listener is running. Waiting for events...\n");
    
    // Process incoming logs
    while let Some(log) = logs_subscription.next().await {
        if let Err(e) = process_log_entry(log.value).await {
            log::error!("Error processing log entry: {}", e);
        }
    }
    
    Ok(())
}

async fn process_log_entry(logs: solana_client::rpc_response::RpcLogsResponse) -> Result<()> {
    // Check if transaction was successful
    if logs.err.is_some() {
        return Ok(()); // Skip failed transactions
    }
    
    // Parse logs to find event data
    for log_line in &logs.logs {
        // Look for program data logs (these contain event data)
        if log_line.starts_with("Program data:") || log_line.contains("Program log:") {
            // Try to extract base64 encoded event data
            if let Some(event_data) = extract_event_data(log_line) {
                match parse_event(&event_data) {
                    Ok(event) => {
                        // Print the event
                        print_event(&event, &logs.signature, 0);
                        
                        // TODO: Store in database
                        // if let Some(db) = db_pool {
                        //     store_event_in_db(db, &event, &logs.signature).await?;
                        // }
                    }
                    Err(e) => {
                        log::debug!("Failed to parse event: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn extract_event_data(log_line: &str) -> Option<Vec<u8>> {
    // Try to extract base64 data from "Program data: <base64>" format
    if log_line.starts_with("Program data:") {
        let parts: Vec<&str> = log_line.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(decoded) = base64::decode(parts[2]) {
                return Some(decoded);
            }
        }
    }
    
    // Try alternative formats
    if log_line.contains("Program log:") {
        // Look for base64 encoded data in log messages
        let parts: Vec<&str> = log_line.split(':').collect();
        if let Some(data_part) = parts.last() {
            let trimmed = data_part.trim();
            if let Ok(decoded) = base64::decode(trimmed) {
                if decoded.len() >= 8 {
                    return Some(decoded);
                }
            }
        }
    }
    
    None
}

