//! Defines the set of API entrypoints that can be called on client side.

use crate::{conn::Hub, store::Store, util::token::JwtToken, Config};
use std::sync::Arc;

mod auth;
mod dto;
mod event;
mod extractor;
mod friend;
mod member;
mod message;
mod room;
mod router;
mod user;
mod websocket;

pub use dto::*;
pub use router::make_app;

/// The data that is shared across the processes.
pub struct AppState {
    db: Store,
    hub: Hub,
    jwt: JwtToken,
    config: Config,
}

impl AppState {
    async fn new(config: Config) -> Arc<Self> {
        let state = AppState {
            db: Store::new(&config).await,
            hub: Hub::default(),
            jwt: JwtToken::new(&config),
            config,
        };

        Arc::new(state)
    }
}
