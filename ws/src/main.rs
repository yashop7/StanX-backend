use std::{collections::HashSet, sync::{Arc, Mutex}};

use actix_web::{
    App, HttpRequest, HttpServer, rt,
    web::{self, Payload},
};
use actix_ws::Message;
use snowflake::ProcessUniqueId;
use tokio::sync::mpsc;

use crate::room_manager::{RoomManager, User, WsMessages};
pub mod room_manager;
pub mod types;

pub async fn ws_handler(
    req: HttpRequest,
    body: Payload,
    room_manager: web::Data<Mutex<RoomManager>>,
) -> Result<actix_web::HttpResponse, actix_web::error::Error> {
    let (tx, mut rx) = mpsc::channel::<String>(32);
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    let user_session = Arc::new(Mutex::new(session));
    let session_clone = user_session.clone();
    let id = ProcessUniqueId::new();
    if let Ok(mut manager) = room_manager.lock() {
        manager.clients.insert(id, User { tx , subscriptions : HashSet::new() });
    }

    rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.recv().await {
            match msg {
                Message::Ping(bytes) => {
                    if user_session.lock().unwrap().pong(&bytes).await.is_err() {
                        return;
                    }
                }
                Message::Text(msg) => {
                    print!("msg: {}",&msg);
                    let message = serde_json::from_str::<WsMessages>(&msg);
                    print!("message : {:?}",&message);
                    if let Ok(message) = message {
                        match message {
                            WsMessages::JoinRoom(room_id) => {
                                print!("{}",&room_id);
                                if let Ok(mut manager) = room_manager.lock() {
                                    let _ = manager
                                        .subscriptions
                                        .entry(room_id.clone())
                                        .or_default()
                                        .insert(id);
                                    if let Some(user) = manager.clients.get_mut(&id) {
                                        user.subscriptions.insert(room_id);
                                    }
                                }
                            }
                            WsMessages::SendMessage(room_id, msg) => {
                                if let Ok(room_manager) = room_manager.lock() {
                                    let _ = room_manager.broadcast(&room_id, msg).await;
                                }
                            }
                            WsMessages::LeaveRoom(room_id) => {
                                if let Ok(mut room_manager) = room_manager.lock() {
                                        if let Some(list) = room_manager.subscriptions.get_mut(&room_id) {
                                            let _ = list.remove(&id);
                                        }
                                }
                            }
                            WsMessages::LeaveRooms => {
                                if let Ok(mut room_manager) = room_manager.lock() {
                                    // Have to make the duplicate of subscription
                                    let subs_list: Vec<String> = room_manager.clients.get(&id).map(|s| s.subscriptions.iter().cloned().collect()).unwrap_or_default();
                                    for sub in subs_list {
                                        if let Some(list) = room_manager.subscriptions.get_mut(&sub) {
                                            let _ = list.remove(&id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => break,
            }
        }
    });

    rt::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(mut session) = session_clone.lock() {
                let _ = session.text(msg);
            }

        }
    });

    Ok(response)

    // One for Sending the message to the room
    // Other is for processing the message and receiving each and every message
}

#[actix_web::main]
pub async fn main() {
    let room_manager = web::Data::new(Mutex::new(RoomManager::new()));
    let _ = HttpServer::new(move || {
        App::new()
            .app_data(room_manager.clone())
            .route("/ws", web::get().to(ws_handler))
    })
    .bind("0.0.0.0:3000")
    .unwrap()
    .run()
    .await;
}