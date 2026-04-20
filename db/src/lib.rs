use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

pub mod models;

pub struct Db {
    pub pool: PgPool,
}

impl Db {
    pub async fn new(db_url : &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&db_url)
            .await?;

        Ok(Self { pool })
    }
}