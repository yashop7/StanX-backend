use std::env;

use axum::{extract::State, http::StatusCode, Json};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::state::state::AppState;

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
    State(app_state): State<AppState>,
    Json(body): Json<UserCreateBodyReq>,
) -> Result<Json<UserCreateResponse>, (StatusCode, String)> {
    // Mapping the error
    let user = app_state.db
        .create_user(&body.username, &body.password)
        .await
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;

    Ok(Json(UserCreateResponse { id: user.id }))
}

pub async fn signin(
    State(app_state): State<AppState>,
    Json(body): Json<UserCreateBodyReq>,
) -> Result<Json<UserSignInResponse>, (StatusCode, String)> {
    let user = app_state.db
        .get_user_by_username(&body.username)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    if user.password != body.password {
        // Hash it Later
        return Err((StatusCode::UNAUTHORIZED, "Wrong Password".to_string()));
    }

    let token = encode(
        &Header::default(),
        &Claims::new(user.id),
        &EncodingKey::from_secret(
            env::var("SECRET_KEY")
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?.as_bytes(),
        ),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UserSignInResponse { token }))
}

