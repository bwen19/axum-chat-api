//! Defines the methods for data storage.

use crate::Config;
use redis::{Client, Commands};
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
            redis::Client::open(config.redis_url.as_ref()).expect("cannot create redis client");
        // check the connection
        let mut con = client
            .get_connection()
            .expect("failed to get redis connection");
        let _: () = con.set("test", "hi").expect("failed to connect redis");

        let store = Self { pool, client };
        store.init().await;

        store
    }
}
