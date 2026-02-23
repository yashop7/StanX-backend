use std::env;

use actix_web::web::{Data, Json};
use db::Db;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct UserCreateBodyReq {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct UserCreateResponse {
    pub id: String,
}

#[derive(Deserialize, Serialize)]
pub struct UserSignInResponse {
    pub token: String,
}

#[derive(Deserialize, Serialize)]
pub struct Claims {
    sub: String,
    exp: usize,
}

impl Claims {
    pub fn new(sub: String) -> Self {
        return Self {
            sub,
            exp: 100000000000,
        };
    }
}

pub async fn create_user(
    db: Data<Db>,
    body: Json<UserCreateBodyReq>,
) -> Result<Json<UserCreateResponse>, actix_web::error::Error> {
    // Mapping the error
    let user = db
        .create_user(&body.username, &body.password)
        .await
        .map_err(|e| actix_web::error::ErrorConflict(e.to_string()))?;

    Ok(Json(UserCreateResponse { id: user.id }))
}

pub async fn signin(
    db: Data<Db>,
    body: Json<UserCreateBodyReq>,
) -> Result<Json<UserSignInResponse>, actix_web::error::Error> {
    let user = db
        .get_user_by_username(&body.username)
        .await
        .map_err(|e| actix_web::error::ErrorUnauthorized(e.to_string()))?;

    if user.password != body.password {
        // Hash it Later
        return Err(actix_web::error::ErrorUnauthorized("Wrong Password"));
    }

    let token = encode(
        &Header::default(),
        &Claims::new(user.id),
        &EncodingKey::from_secret(
            env::var("SECRET_KEY")
                .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?.as_bytes(),
        ),
    )
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(Json(UserSignInResponse { token }))
}
