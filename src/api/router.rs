//! Defines the router of the server.

use super::{auth, message, room, user, websocket, AppState};
use crate::Config;
use axum::{
    error_handling::HandleErrorLayer,
    http::{header, Method, StatusCode},
    BoxError, Router,
};
use std::time::Duration;
use tower::{timeout::TimeoutLayer, ServiceBuilder};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Create router of the application.
///
/// - `config`: The global configure of the application.
pub async fn make_app(config: Config) -> Router {
    let state = AppState::new(config).await;

    Router::new()
        .merge(websocket::router())
        .nest(
            "/api",
            auth::router()
                .merge(user::router())
                .merge(message::router())
                .merge(room::router()),
        )
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
                .layer(
                    CorsLayer::new()
                        .allow_methods([Method::GET, Method::POST])
                        .allow_origin(Any)
                        .allow_headers([header::CONTENT_TYPE]),
                ),
        )
        .with_state(state)
}
