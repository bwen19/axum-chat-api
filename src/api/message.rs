//! Handlers for messages

use super::{
    dto::{InitializeResponse, NewMessageRequest, SendFileResponse},
    event::ServerEvent,
    extractor::AuthGuard,
    AppState, NewMessageResponse,
};
use crate::{
    conn::Client,
    core::{
        constant::{IMAGE_KEY, KIND_FILE, KIND_IMAGE},
        Error,
    },
    util,
};
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
use validator::Validate;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/message/file", post(send_file))
        .layer(DefaultBodyLimit::max(150 * 1024 * 1024))
}

async fn send_file(
    State(state): State<Arc<AppState>>,
    AuthGuard(claims): AuthGuard,
    mut multipart: Multipart,
) -> Result<Json<SendFileResponse>, Error> {
    if let Some(field) = multipart.next_field().await? {
        let file_name = if let Some(file_name) = field.file_name() {
            file_name.trim().to_owned()
        } else {
            return Err(Error::BadRequest);
        };

        let kind = if let Some(content_type) = field.content_type() {
            if content_type.starts_with(IMAGE_KEY) {
                KIND_IMAGE.to_owned()
            } else {
                KIND_FILE.to_owned()
            }
        } else {
            KIND_FILE.to_owned()
        };

        let file_url = util::common::generate_file_name(claims.user_id);
        let path = Path::new(&state.config.public_directory).join(&file_url[1..]);

        if let Err(e) = stream_to_file(&path, field).await {
            if path.is_file() {
                tokio::fs::remove_file(&path).await?;
            }
            return Err(e);
        }

        let rsp = SendFileResponse {
            content: file_name,
            file_url,
            kind,
        };
        return Ok(Json(rsp));
    }
    Err(Error::BadRequest)
}

// Save a `Stream` to a file
async fn stream_to_file<S, E>(path: &PathBuf, stream: S) -> Result<(), Error>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    async {
        // convert the stream into an `AsyncRead`.
        let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        // copy the body into the file.
        let mut file = BufWriter::new(File::create(path).await?);
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, io::Error>(())
    }
    .await?;
    Ok(())
}

pub async fn initialize(state: &Arc<AppState>, client: &Client) -> Result<(), Error> {
    // get room information from database
    let rooms = state.db.get_user_rooms(client.user_id()).await?;
    let friends = state.db.get_user_friends(client.user_id()).await?;

    // create connections to the room channels
    let rooms_id: Vec<i64> = rooms.iter().map(|room| room.id).collect();
    state.hub.connect(client, rooms_id).await?;

    // send rooms and friends info to the client socket
    let rsp = InitializeResponse { rooms, friends };
    let msg = ServerEvent::Initialize(rsp).to_msg()?;
    client.send(msg).await?;

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
        return Err(Error::Forbidden);
    }

    // create message and store in redis
    let message = state.db.cache_message(client.user_id(), req).await?;
    let room_id = message.room_id;

    // send message to the room
    let rsp = NewMessageResponse { message };
    let msg = ServerEvent::NewMessage(rsp).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    Ok(())
}
