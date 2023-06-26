pub mod friend;
pub mod invitation;
pub mod member;
pub mod message;
pub mod model;
pub mod room;
pub mod session;
pub mod user;

use crate::{api::user::CreateUserRequest, error::AppError, Config};
use sqlx::{postgres::PgPoolOptions, PgPool};

// ========================// Store //======================== //

#[derive(Debug, Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn new(config: &Config) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.db_url)
            .await
            .expect("failed to connect database");

        let store = Self { pool };
        init_db(&store).await;
        store
    }
}

async fn init_db(store: &Store) {
    sqlx::migrate!()
        .run(&store.pool)
        .await
        .expect("failed to run migrate up");
    tracing::info!("db migrated successfully");

    let req = CreateUserRequest {
        username: "admin".to_owned(),
        password: "098765".to_owned(),
        role: "admin".to_owned(),
    };

    if let Err(e) = store.create_user(&req.into()).await {
        match e {
            AppError::UniqueConstraint(_) => tracing::info!("Admin has already been created"),
            _ => panic!("failed to create admin"),
        }
    };
}
