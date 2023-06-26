mod channel;
mod event;
mod handler;
mod user_socket;

use crate::AppState;
use axum::{routing::get, Router};
pub use channel::ChannelManager;
use std::sync::Arc;

// ========================// WebSocket Router //======================== //

/// Create ws router
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(handler::ws_handler))
}
