use super::Store;
use crate::{api::CreateUserRequest, core::Error, Config};
use sqlx::postgres::PgPoolOptions;

impl Store {
    pub async fn new(config: &Config) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(16)
            .connect(&config.database_url)
            .await
            .expect("cannot connect to database");

        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let store = Self { pool, client };
        store.init().await;

        store
    }

    pub async fn init(&self) {
        sqlx::migrate!()
            .run(&self.pool)
            .await
            .expect("failed to run migrate up");
        tracing::info!("db migrated successfully");

        let arg = CreateUserRequest {
            username: "admin".to_owned(),
            password: "098765".to_owned(),
            role: "admin".to_owned(),
        };

        if let Err(e) = self.create_user(&arg).await {
            match e {
                Error::UniqueConstraint(_) => tracing::info!("admin has already been created"),
                _ => panic!("failed to create admin account"),
            }
        };
    }
}
