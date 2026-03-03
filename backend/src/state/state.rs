use db::Db;
use std::sync::Arc;

// This is our AppState
// DB 
// Connections of Channel
// markets WS
// Something take Pvt.keys of the user and we can call the functions easily from the backend
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>
}