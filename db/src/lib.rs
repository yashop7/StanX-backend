use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};

pub mod models;

pub struct Db {
    pub pool: PgPool,
}

impl Db {
    pub async fn new(db_url : &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        Ok(Self { pool })
    }
}