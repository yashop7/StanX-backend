use common::{OrderbookDiff, OrderbookState};
use db::Db;
use tokio::sync::{broadcast};
use std::{collections::HashMap, sync::{Arc, RwLock}};

// This is our AppState
// DB 
// Connections of Channel
// markets WS
// Something take Pvt.keys of the user and we can call the functions easily from the backend


pub enum MessagetoEngine {
    LimitOrder {

    },
    MarketOrder {

    },
    CancelOrder {
        order_id : String
        
    }, 

}

#[derive(Clone)]
pub struct AppState {
    // Like here what messages to send to the market, what can be
    // Like can be Split, Merge, Market, Limit, Cancel, 
    pub orderbook : Arc<RwLock<HashMap<i32, Arc<OrderbookState>>>>,
    pub ob_channels : Arc<RwLock<HashMap<i32, broadcast::Sender<OrderbookDiff>>>>,
    pub db: Arc<Db>
}
