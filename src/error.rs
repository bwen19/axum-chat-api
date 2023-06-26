use axum::{
    extract::rejection::{JsonRejection, QueryRejection, TypedHeaderRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::convert::Infallible;
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};

// ========================// AppError //======================== //

/// A common error type that can be used throughout the App
#[derive(Error, Debug)]
pub enum AppError {
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
    #[error("Cannot delete yourself")]
    DeleteUserSelf,
    #[error("The invitation code is invalid")]
    InvitationCode,
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
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Failed to read or write file")]
    CustomIo,
    #[error("Token creation error")]
    TokenCreation,
    #[error("Failed to convert timestamp to DateTime")]
    TimeConversion,

    // Websocket Error
    #[error("You are not in this room")]
    NotInRoom,
    #[error("Failed to send websocket message")]
    SendMessage,
    #[error("Failed to serialize websocket message")]
    SerializeMessage,
}

// Convert broadcast send error to AppError
impl<T> From<broadcast::error::SendError<T>> for AppError {
    fn from(_: broadcast::error::SendError<T>) -> Self {
        Self::SendMessage
    }
}

// Convert mpsc send error to AppError
impl<T> From<mpsc::error::SendError<T>> for AppError {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::SendMessage
    }
}

impl AppError {
    pub fn into_error(self) -> (StatusCode, String) {
        let status = match self {
            // 203
            AppError::ExpiredToken => StatusCode::NON_AUTHORITATIVE_INFORMATION,
            // 400
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::UserNotExist => StatusCode::BAD_REQUEST,
            AppError::WrongPassword => StatusCode::BAD_REQUEST,
            AppError::UniqueConstraint(_) => StatusCode::BAD_REQUEST,
            AppError::DeleteUserSelf => StatusCode::BAD_REQUEST,
            AppError::InvitationCode => StatusCode::BAD_REQUEST,
            AppError::InvalidFile => StatusCode::BAD_REQUEST,
            // 401
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            // 403
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotInRoom => StatusCode::FORBIDDEN,
            // 404
            AppError::NotFound => StatusCode::NOT_FOUND,
            // 422
            AppError::QueryRejection(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::JsonRejection(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::TypedHeaderRejection(_) => StatusCode::UNPROCESSABLE_ENTITY,
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

// Axum allows you to return AppError which impl IntoResponse
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        self.into_error().into_response()
    }
}

// ========================// AppResult //======================== //

/// Custom Result type
pub type AppResult<T> = Result<T, AppError>;

/// A helper trait for easily converting SqlxError into AppError
pub trait ResultExt<T> {
    /// If self contains a database constraint error with the given name,
    /// transform the error
    fn on_constraint(self, name: &str, value: &str) -> AppResult<T>;

    /// Handle not found error by fetching exactly one
    fn exactly_one(self) -> AppResult<T>;
}

impl<T> ResultExt<T> for Result<T, sqlx::Error> {
    fn on_constraint(self, name: &str, value: &str) -> AppResult<T> {
        self.map_err(|e| match e {
            sqlx::Error::Database(dbe) if dbe.constraint() == Some(name) => {
                AppError::UniqueConstraint(format!("{}: `{}` has been taken", name, value))
            }
            sqlx::Error::RowNotFound => AppError::NotFound,
            _ => AppError::Sqlx(e),
        })
    }

    fn exactly_one(self) -> AppResult<T> {
        self.map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound,
            _ => AppError::Sqlx(e),
        })
    }
}
