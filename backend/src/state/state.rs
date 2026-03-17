use db::Db;
use tokio::sync::mpsc;
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
    pub markets : Arc<RwLock<HashMap<String, mpsc::Sender<MessagetoEngine>>>>,
    pub db: Arc<Db>
}

type State = AppState;