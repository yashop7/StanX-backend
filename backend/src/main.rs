use actix_web::{App, HttpServer, web};
use db::Db;
use dotenvy;

use crate::routes::user::{create_user, signin};
pub mod routes;
pub mod middleware;

#[actix_web::main]
async fn main() {
    dotenvy::from_filename("backend/.env").ok();
    let db = web::Data::new(Db::new().await.unwrap());
    let _ = HttpServer::new(move || {
        App::new()
            .service(web::resource("/signup").route(web::post().to(create_user)))
            .service(web::resource("/signin").route(web::post().to(signin)))
            .app_data(db.clone())
    })
    .bind("0.0.0.0:3000")
    .unwrap()
    .run()
    .await;
}
