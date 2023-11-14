//! Defines the methods for data storage.

use crate::Config;
use redis::Client;
use sqlx::{postgres::PgPoolOptions, PgPool};

mod friend;
mod init;
mod member;
mod message;
mod model;
mod room;
mod session;
mod user;

pub use model::*;

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
    client: Client,
}

impl Store {
    pub async fn new(config: &Config) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(16)
            .connect(&config.database_url)
            .await
            .expect("cannot connect to database");

        let client =
            redis::Client::open(config.redis_url.as_ref()).expect("cannot connect to redis");

        let store = Self { pool, client };
        store.init().await;

        store
    }
}
