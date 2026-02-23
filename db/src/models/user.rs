use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::Db;

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub id: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetUserRequest {
    username: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetUserResponse {
    pub user: User,
}

impl Db {
    pub async fn create_user(
        &self,
        username: &String,
        password: &String,
    ) -> Result<CreateUserResponse> {
        let user = sqlx::query_as!(
            User,
            "INSERT INTO users(username,password) VALUES($1, $2) RETURNING id, username, password",
            username,
            password
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(CreateUserResponse { id: user.id })
    }

    pub async fn get_user_by_username(&self, username: &String) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, password FROM users WHERE username=$1",
            username
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }
}
