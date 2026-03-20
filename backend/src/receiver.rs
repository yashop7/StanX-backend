// We will listen to the PubSub receiver
// We will change the Orderbook Pointer
// Send the Orderbook Diff to the Market WS tx

pub async fn run(state: AppState, redis_url: String) {
    let redis_client = redis::Client::open(redis_url.clone())
        .map_err(|e| anyhow::anyhow!("Failed to connect to Redis at {}: {}", redis_url, e))?;

    let mut publisher = redis_client.get_connection()?;
    let pub_sub = publisher.as_pubsub();

    match pub_sub.subscribe("ob")
}
