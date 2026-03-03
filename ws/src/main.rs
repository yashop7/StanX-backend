use actix_web::*;
use actix_web::{
    App, HttpRequest, HttpServer, rt,
    web::{self, Payload},
};
use actix_ws::Message;
use snowflake::ProcessUniqueId;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::room_manager::{RoomManager, User};
pub mod room_manager;
pub mod types;

async fn ws_handler(
    request: HttpRequest,
    body: Payload,
    room_manager: web::Data<Arc<Mutex<RoomManager>>>,
) -> Result<actix_web::HttpResponse, actix_web::error::Error> {
    let (response, session, mut stream) = actix_ws::handle(&request, body)?;
    let session = Arc::new(Mutex::new(session));
    let session_clone = session.clone();
    let (tx, mut rx) = mpsc::channel::<String>(32);

    let id = ProcessUniqueId::new();
    room_manager.lock().await.clients.insert(id, User { tx });

    rt::spawn(async move {
        while let Some(Ok(message)) = stream.recv().await {
            match message {
                Message::Ping(data) => {
                    let _ = session_clone.lock().await.pong(&data).await;
                }
                Message::Text(message) => {
                    let message = serde_json::from_str(&message);
                    if let Ok(message) = message {
                        match message {
                            types::Message::JoinRoom(room_id) => {
                                let mut room_manager = room_manager.lock().await;
                                room_manager
                                    .subscriptions
                                    .entry(room_id)
                                    .or_default()
                                    .insert(id);
                            }
                            types::Message::LeaveRoom(room_id) => {
                                let mut room_manager = room_manager.lock().await;
                                if let Some(users) = room_manager.subscriptions.get_mut(&room_id) {
                                    users.remove(&id);
                                }
                            }
                            types::Message::SendMessage(room_details) => {
                                println!("{:?}", room_details.clone());
                                let _ = room_manager
                                    .lock()
                                    .await
                                    .broadcast(room_details.room_id, room_details.message)
                                    .await;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    });

    rt::spawn(async move {
        while let Some(message) = rx.recv().await {
            let _ = session.lock().await.text(message).await;
        }
    });

    Ok(response)
}

#[actix_web::main]
async fn main() {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => println!("1970-01-01 00:00:00 UTC was {} seconds ago!", n.as_secs()),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }

    let room_manager = Arc::new(Mutex::new(RoomManager::new()));
    let _ = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(room_manager.clone()))
            .route("/ws", web::get().to(ws_handler))
    })
    .bind("0.0.0.0:3000")
    .unwrap()
    .run()
    .await;
}