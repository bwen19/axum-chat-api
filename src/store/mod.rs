//! Defines the set of API entrypoints that can be called on client side.

use redis::Client;
use sqlx::PgPool;

mod friend;
mod init;
mod member;
mod message;
mod model;
mod room;
mod session;
mod user;

pub use model::*;

/// The data that is shared across the processes.
#[derive(Clone)]
pub struct Store {
    pool: PgPool,
    client: Client,
}
