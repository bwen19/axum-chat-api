//! Defines the router of the server.

use super::{auth, message, user, websocket, AppState};
use crate::Config;
use axum::{http::header::COOKIE, Router};
use std::iter::once;
use tower_http::{sensitive_headers::SetSensitiveRequestHeadersLayer, trace::TraceLayer};

/// Create router of the application.
///
/// - `config`: The global configure of the application.
pub async fn make_app(config: Config) -> Router {
    let state = AppState::new(config).await;

    Router::new()
        .merge(websocket::router())
        .nest(
            "/api",
            auth::router().merge(user::router().merge(message::router())),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(SetSensitiveRequestHeadersLayer::new(once(COOKIE)))
}
