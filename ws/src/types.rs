use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct RoomMessage {
    // #[serde(rename = "RoomId")] can rename the variable
    pub room_id : String,
    pub message : String
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    JoinRoom(String),
    LeaveRoom(String),
    #[serde(rename = "Message")]
    SendMessage(RoomMessage)
}