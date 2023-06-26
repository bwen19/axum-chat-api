use super::validator as VAL;
use crate::{
    db::model::{FriendInfo, MessageInfo, RoomInfo},
    error::{AppError, AppResult},
    extractor::AuthGuard,
    util, AppState,
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Multipart, State},
    routing::post,
    BoxError, Json, Router,
};
use futures::{Stream, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{fs::File, io::BufWriter};
use tokio_util::io::StreamReader;
use tower_http::limit::RequestBodyLimitLayer;
use validator::Validate;

// ========================// Message Router //======================== //

/// Create user router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/message/file", post(send_file))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            150 * 1024 * 1024, /* 150mb */
        ))
}

// ========================// Message //======================== //

// ---------------- Initialize ---------------- //
/// Used to get initial rooms and friends
#[derive(Deserialize, Validate)]
pub struct InitialRequest {
    #[validate(range(min = 1, message = "invalid timestamp"))]
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct InitialResponse {
    pub rooms: Vec<RoomInfo>,
    pub friends: Vec<FriendInfo>,
}

// ---------------- New message ---------------- //
#[derive(Deserialize, Validate)]
pub struct NewMessageRequest {
    #[validate(range(min = 1, message = "invalid ID"))]
    pub room_id: i64,
    #[validate(length(min = 1, max = 500, message = "must be between 1 and 500 characters"))]
    pub content: String,
    #[validate(custom = "VAL::validate_message_kind")]
    pub kind: String,
}

/// Used to pass a single message to client
#[derive(Serialize)]
pub struct NewMessageResponse {
    pub room_id: i64,
    pub message: MessageInfo,
}

// ========================// Send File //======================== //

#[derive(Serialize)]
pub struct SendFileResponse {
    pub file_url: String,
}

async fn send_file(
    State(state): State<Arc<AppState>>,
    AuthGuard(_): AuthGuard,
    mut multipart: Multipart,
) -> AppResult<Json<SendFileResponse>> {
    let file_url = if let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = if let Some(file_name) = field.file_name() {
            file_name.to_owned()
        } else {
            return Err(AppError::InvalidFile);
        };

        let file_url = util::common::generate_file_name(&file_name);
        let path = Path::new(&state.config.public_directory).join(&file_url[1..]);

        if let Err(e) = stream_to_file(&path, field).await {
            if path.is_file() {
                tokio::fs::remove_file(&path).await?;
            }
            return Err(e);
        }

        file_url
    } else {
        return Err(AppError::InvalidFile);
    };

    Ok(Json(SendFileResponse { file_url }))
}

// Save a `Stream` to a file
async fn stream_to_file<S, E>(path: &PathBuf, stream: S) -> AppResult<()>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    async {
        // Convert the stream into an `AsyncRead`.
        let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        // Copy the body into the file.
        let mut file = BufWriter::new(File::create(path).await?);
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, io::Error>(())
    }
    .await
    .map_err(|_| AppError::CustomIo)
}
