use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

pub mod models;

pub struct Db {
    pub pool: PgPool,
}

impl Db {
    pub async fn new() -> Result<Self> {
        let db_url = env::var("DATABASE_URL")?;

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        Ok(Self { pool })
    }
}