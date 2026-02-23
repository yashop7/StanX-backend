use std::{collections::{HashMap, HashSet}};
use tokio::sync::mpsc::Sender;
use snowflake::ProcessUniqueId; // Generated unique IDs
pub struct User {
    pub tx : Sender<String>
}

pub struct RoomManager {
    pub clients : HashMap<ProcessUniqueId, User>,
    pub subscriptions : HashMap<String, HashSet<ProcessUniqueId>>
}

impl RoomManager {
    pub fn new() -> Self {
        Self { clients: HashMap::new(), subscriptions: HashMap::new() }
    }

    // Broadcasting 
    pub async fn broadcast(&self, room_id : String , message : String) {
        let subscriptions = self.subscriptions.get(&room_id);
        if let Some(subscription) = subscriptions {
            for client_id in subscription {
                if let Some(user) = self.clients.get(client_id) {
                    let _ = user.tx.send(message.clone()).await;
                } 
            }
        }
    }
}