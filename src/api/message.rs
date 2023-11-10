//! Handlers for messages

use super::{
    event::ServerEvent,
    extractor::AuthGuard,
    AppState, {InitializeRequest, InitializeResponse, NewMessageRequest, SendFileResponse},
};
use crate::{conn::Client, core::Error, util};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Multipart, State},
    routing::post,
    BoxError, Json, Router,
};
use futures::{Stream, TryStreamExt};
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

async fn send_file(
    State(state): State<Arc<AppState>>,
    AuthGuard(_): AuthGuard,
    mut multipart: Multipart,
) -> Result<Json<SendFileResponse>, Error> {
    let file_url = if let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = if let Some(file_name) = field.file_name() {
            file_name.to_owned()
        } else {
            return Err(Error::InvalidFile);
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
        return Err(Error::InvalidFile);
    };

    Ok(Json(SendFileResponse { file_url }))
}

// Save a `Stream` to a file
async fn stream_to_file<S, E>(path: &PathBuf, stream: S) -> Result<(), Error>
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
    .map_err(|_| Error::CustomIo)
}

pub async fn initialize(
    state: &Arc<AppState>,
    client: &Client,
    req: InitializeRequest,
) -> Result<(), Error> {
    req.validate()?;

    // get room information from database
    let rooms = state
        .db
        .get_user_rooms(client.user_id(), req.timestamp)
        .await?;
    let friends = state.db.get_user_friends(client.user_id()).await?;

    // create connections to the room channels
    let room_ids: Vec<i64> = rooms.iter().map(|room| room.id).collect();
    state.hub.connect(client, room_ids).await?;

    // send rooms and friends info to the client socket
    let rsp = InitializeResponse { rooms, friends };
    let msg = ServerEvent::Initialized(rsp).to_msg()?;
    client.send(msg).await;

    Ok(())
}

pub async fn send_message(
    state: &Arc<AppState>,
    client: &Client,
    req: NewMessageRequest,
) -> Result<(), Error> {
    req.validate()?;

    // check whether user is in the room
    if !state.hub.is_user_in(client.user_id(), req.room_id).await {
        return Err(Error::NotInRoom);
    }

    // create message and store in database
    let data = state.db.create_message(client.user_id(), &req).await?;

    // send message to the room
    let room_id = data.room_id;
    let msg = ServerEvent::NewMessage(data).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    Ok(())
}
