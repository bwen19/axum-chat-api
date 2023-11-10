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
    #[error("The user doesn't exist")]
    UserNotExist,
    #[error("The password doesn't match")]
    WrongPassword,
    #[error("Duplicate value on {0}")]
    UniqueConstraint(String),
    #[error("The friend status is invalid")]
    FriendStatus,
    #[error("Cannot delete yourself")]
    DeleteUserSelf,
    #[error("The uploaded file is invalid")]
    InvalidFile,

    // 401 Unauthorized
    #[error("Authentication required")]
    Unauthorized,

    // 403 Forbidden
    #[error("You don't have permission to access")]
    Forbidden,

    // 404 NotFound
    #[error("Resource not found")]
    NotFound,

    // 422 UnprocessableEntity
    #[error(transparent)]
    QueryRejection(#[from] QueryRejection),
    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),
    #[error(transparent)]
    TypedHeaderRejection(#[from] TypedHeaderRejection),

    // 500 Internal Server Error
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
    #[error("{0}")]
    Internal(String),
    #[error("Failed to read or write file")]
    CustomIo,
    #[error(transparent)]
    JwtToken(#[from] jsonwebtoken::errors::Error),

    // Websocket Error
    #[error("You are not in this room")]
    NotInRoom,
    #[error("Failed to send websocket message")]
    SendMessage,
    #[error("Failed to serialize websocket message")]
    SerializeMessage,
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
            Error::Validation(_) => StatusCode::BAD_REQUEST,
            Error::UserNotExist => StatusCode::BAD_REQUEST,
            Error::WrongPassword => StatusCode::BAD_REQUEST,
            Error::UniqueConstraint(_) => StatusCode::BAD_REQUEST,
            Error::DeleteUserSelf => StatusCode::BAD_REQUEST,
            Error::InvalidFile => StatusCode::BAD_REQUEST,
            // 401
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            // 403
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::NotInRoom => StatusCode::FORBIDDEN,
            // 404
            Error::NotFound => StatusCode::NOT_FOUND,
            // 422
            Error::QueryRejection(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::JsonRejection(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::TypedHeaderRejection(_) => StatusCode::UNPROCESSABLE_ENTITY,
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
