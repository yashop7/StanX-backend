// This is room manager , eans It will maage all rooms, Like 
// It will store room details and what users are in which Room

use std::collections::{HashMap, HashSet};

use snowflake::ProcessUniqueId;
use tokio::sync::mpsc::Sender;
use serde::*;
use crate::room_manager;

#[derive(Clone)]
pub struct User {
    pub tx : Sender<String>,
    pub subscriptions: HashSet<String>,
}

// It will store the Tx of the Users
#[derive(Debug, Serialize , Deserialize)]
pub enum WsMessages {
    JoinRoom(String), // RoomId
    SendMessage(String, String), // RoomId, message
    LeaveRoom(String), // The room you want to leave
    LeaveRooms
}


#[derive(Clone)]
pub struct RoomManager {
    pub clients : HashMap<ProcessUniqueId, User>,
    pub subscriptions : HashMap<String , HashSet<ProcessUniqueId>>
}

//Room Manager
/// Broadcasts a message to all clients subscribed to a specific room.
///
/// # Arguments
///
/// * `room_id` - The ID of the room to broadcast the message to
/// * `message` - The message string to send to all subscribed clients
///
/// # Returns
///
/// * `Ok(())` - Successfully broadcasted the message to all clients in the room
/// * `Err(String)` - Returns an error if the room doesn't exist in subscriptions
///
/// # Error Handling
///
/// # Examples
///
/// Using `ok_or()` for static error messages:
/// ```ignore
/// let users = self.subscriptions.get(room_id)
///     .ok_or("Room not found".to_string())?;
/// ```
///
/// Using `map_err()` for custom error transformation:
/// ```ignore
/// let users = self.subscriptions.get(room_id)
///     .map_err(|_| format!("Room {} not found", room_id))?;
/// ```
///
/// Using `match` for explicit handling:
/// ```ignore
/// let users = match self.subscriptions.get(room_id) {
///     Some(u) => u,
///     None => return Err(format!("Room {} not found", room_id)),
/// };
/// ```
///
/// Using `unwrap_or()` with default value:
/// ```ignore
/// let users = self.subscriptions.get(room_id).unwrap_or(&HashSet::new());
/// ```
///
/// Using `map()` to transform Option:
/// ```ignore
/// let user_count = self.subscriptions.get(room_id).map(|u| u.len()).unwrap_or(0);
/// ```
///
/// # Note
///
/// Failed message sends are silently ignored with `let _ = ...` to prevent one failed client
/// from interrupting broadcasts to other clients


impl RoomManager  {
    
    pub fn new() -> Self {
        Self { clients: HashMap::new(), subscriptions: HashMap::new() }
    }

    // pub fn register_user(&self, room_id : String) -> Result<()> {

    //     Ok(())
    // }

    pub async fn broadcast(&self, room_id: &str, message: String) -> Result<(), String> {
        let users = self.subscriptions.get(room_id)
            .ok_or_else(|| format!("Room {} not found", room_id))?;

        for user_id in users {
            if let Some(user) = self.clients.get(user_id) {
                let _ = user.tx.send(message.clone()).await;
            }
        }
        Ok(())
    }

// We will manage this in Frontend
    // pub fn register_user(&mut self, user_id: ProcessUniqueId, tx: Sender<String>) -> Result<(), String> {
    //     // Let's register a Particular user
    //     self.clients.insert(user_id, v)
        
    //     Ok(())
    // }


}