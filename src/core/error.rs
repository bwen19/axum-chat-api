//! Custom error types

use axum::{
    extract::rejection::{JsonRejection, QueryRejection, TypedHeaderRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::convert::Infallible;
use tokio::sync::{broadcast, mpsc};

/// A common error type that can be used throughout the App
#[derive(thiserror::Error, Debug)]
pub enum Error {
    // 203
    #[error("Token is expired")]
    ExpiredToken,

    // 400 Bad Request
    #[error(transparent)]
    Validation(#[from] validator::ValidationErrors),

    #[error("The password doesn't match")]
    InvalidPassword,

    #[error("Already exist on {0}")]
    UniqueConstraint(String),

    #[error("The friend status is invalid")]
    FriendStatus,

    #[error("Bad request")]
    BadRequest,

    // 401 Unauthorized
    #[error("Authentication required")]
    Unauthorized,

    // 403 Forbidden
    #[error("Permission denied")]
    Forbidden,

    // 404 NotFound
    #[error("Not exists")]
    NotFound,

    // 422 UnprocessableEntity
    #[error(transparent)]
    QueryRejection(#[from] QueryRejection),

    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),

    #[error(transparent)]
    TypedHeaderRejection(#[from] TypedHeaderRejection),

    // 500 Internal Server Error
    #[error("Argon2 internal error")]
    Argon2,

    #[error("Database error while collecting results")]
    Database,

    #[error(transparent)]
    Infallible(#[from] Infallible),

    #[error(transparent)]
    Redis(#[from] redis::RedisError),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JwtToken(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::error::Error),

    #[error("Failed to send websocket message")]
    SendMessage,
}

// Convert broadcast send error to Error
impl<T> From<broadcast::error::SendError<T>> for Error {
    fn from(_: broadcast::error::SendError<T>) -> Self {
        Self::SendMessage
    }
}

// Convert mpsc send error to Error
impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::SendMessage
    }
}

impl Error {
    pub fn into_error(self) -> (StatusCode, String) {
        let status = match self {
            // 203
            Error::ExpiredToken => StatusCode::NON_AUTHORITATIVE_INFORMATION,
            // 400
            Error::Validation(_)
            | Error::InvalidPassword
            | Error::UniqueConstraint(_)
            | Error::FriendStatus
            | Error::BadRequest => StatusCode::BAD_REQUEST,
            // 401
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            // 403
            Error::Forbidden => StatusCode::FORBIDDEN,
            // 404
            Error::NotFound => StatusCode::NOT_FOUND,
            // 422
            Error::QueryRejection(_) | Error::JsonRejection(_) | Error::TypedHeaderRejection(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            _ => {
                tracing::error!("{}", self.to_string());
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Server internal error".into(),
                );
            }
        };
        (status, self.to_string())
    }
}

// Axum allows you to return Error which impl IntoResponse
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.into_error().into_response()
    }
}
