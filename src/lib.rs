mod api;
mod db;
mod error;
mod extractor;
mod util;
mod ws;

use axum::{http::header::COOKIE, Router};
use db::Store;
use std::iter::once;
use std::sync::Arc;
use tower_http::{sensitive_headers::SetSensitiveRequestHeadersLayer, trace::TraceLayer};
pub use util::config::Config;
use ws::ChannelManager;

const COOKIE_NAME: &str = "jwt";

// ========================// AppState //======================== //

/// Shared state throught the App
pub struct AppState {
    config: Config,
    db: Store,
    channel: ChannelManager,
}

impl AppState {
    fn new(config: Config, db: Store) -> Arc<Self> {
        let state = AppState {
            config,
            db,
            channel: ChannelManager::new(),
        };

        Arc::new(state)
    }
}

// ========================// App Router //======================== //

/// Create the router of the App
pub async fn app(config: Config) -> Router {
    let db = Store::new(&config).await;
    let state = AppState::new(config, db);

    api::router()
        .merge(ws::router())
        .layer(TraceLayer::new_for_http())
        .layer(SetSensitiveRequestHeadersLayer::new(once(COOKIE)))
        .with_state(state)
}
