pub mod auth;
pub mod friend;
pub mod invitation;
pub mod member;
pub mod message;
pub mod room;
pub mod user;
pub mod validator;

use crate::AppState;
use axum::Router;
use std::sync::Arc;

// ========================// Api Router //======================== //

/// Create api router
pub fn router() -> Router<Arc<AppState>> {
    Router::new().nest(
        "/api",
        auth::router().merge(
            user::router()
                .merge(message::router())
                .merge(invitation::router()),
        ),
    )
}
