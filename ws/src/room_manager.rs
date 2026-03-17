// This is room manager , eans It will maage all rooms, Like
// It will store room details and what users are in which Room

use std::collections::{HashMap, HashSet};

use crate::room_manager;
use serde::*;
use snowflake::ProcessUniqueId;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct User {
    pub tx: Sender<String>,
    pub subscriptions: HashSet<String>,
}

// It will store the Tx of the Users
#[derive(Debug, Serialize, Deserialize)]
pub enum WsMessages {
    JoinRoom(String),            // RoomId
    SendMessage(String, String), // RoomId, message
    LeaveRoom(String),           // The room you want to leave
    LeaveRooms,
}

#[derive(Clone)]
pub struct RoomManager {
    pub clients: HashMap<ProcessUniqueId, User>,
    pub subscriptions: HashMap<String, HashSet<ProcessUniqueId>>,
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

impl RoomManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            subscriptions: HashMap::new(),
        }
    }

    // pub fn register_user(&self, room_id : String) -> Result<()> {

    //     Ok(())
    // }

    pub async fn broadcast(&self, room_id: &str, message: String) -> Result<(), String> {
        let users = self
            .subscriptions
            .get(room_id)
            .ok_or_else(|| format!("Room {} not found", room_id))?;

        for user_id in users {
            if let Some(user) = self.clients.get(user_id) {
                let _ = user.emit(message.clone());
            }
        }
        Ok(())
    }

    pub fn subscribe_user_for_room(
        &mut self,
        id: ProcessUniqueId,
        room_id: &str,
    ) -> Result<(), String> {
        let subs = self
            .subscriptions
            .entry(room_id.to_string())
            .or_insert_with(HashSet::new);

        if subs.contains(&id) {
            Err(format!("User already subscribed to room {}", room_id))
        } else {
            subs.insert(id);
            Ok(())
        }
    }

    pub fn unsubscribe_user_from_room(
        &mut self,
        id: ProcessUniqueId,
        room_id: &str,
    ) -> Result<(), String> {
        if let Some(list) = self.subscriptions.get_mut(&room_id.to_string()) {
            list.remove(&id);
            Ok(())
        } else {
            Err(format!("Room_Id doesn't exist"))
        }
    }

    pub fn remove_user(&mut self, id: ProcessUniqueId) -> Result<(), String> {
        // remove the user from the clients then subscriptions

        if self.clients.contains_key(&id) {
            self.clients.remove_entry(&id);
            Ok(())
        } else {
            Err(format!("No user found in the client list"))
        }
    }

    // We will manage this in Frontend
    // pub fn register_user(&mut self, user_id: ProcessUniqueId, tx: Sender<String>) -> Result<(), String> {
    //     // Let's register a Particular user
    //     self.clients.insert(user_id, v)

    //     Ok(())
    // }
}

impl User {
    pub fn subscribe(&mut self, room_id: String) {
        self.subscriptions.insert(room_id);
    }

    pub fn unsubscribe(&mut self, room_id: String) -> Result<(), String> {
        if self.subscriptions.contains(&room_id) {
            self.subscriptions.remove(&room_id);
            Ok(())
        } else {
            Err(format!("Room {} not found in subscriptions", room_id))
        }
    }

    pub async fn emit(&self, msg: String) {
        // Right now assuming the msg is in Seralised form
        // Let's see If in future msg comes in desealized form and we
        // we convert it to seralised
        let _ = self.tx.send(msg).await;
    }
}
