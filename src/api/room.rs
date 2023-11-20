//! Handlers for chat rooms

use super::dto::{
    ChangeCoverResponse, DeleteMembersRequest, DeleteMembersResponse, DeleteRoomRequest,
    DeleteRoomResponse, HubStatusResponse, LeaveRoomRequest, NewRoomRequest, NewRoomResponse,
    UpdateRoomResquest,
};
use super::{
    event::ServerEvent,
    extractor::{AdminGuard, CookieGuard},
    AppState,
};
use crate::core::constant::{IMAGE_KEY, PUBLIC_ROOM_COVER, RANK_OWNER};
use crate::{conn::Client, core::Error, util};
use axum::{
    extract::{Multipart, Path, State},
    routing::{get, post},
    Json, Router,
};
use std::{path, sync::Arc};
use validator::Validate;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/room/cover/:room_id", post(change_cover))
        .route("/room/status", get(get_status))
}

async fn get_status(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
) -> Result<Json<HubStatusResponse>, Error> {
    let rsp = state.hub.status().await?;
    Ok(Json(rsp))
}

async fn change_cover(
    State(state): State<Arc<AppState>>,
    CookieGuard(claims): CookieGuard,
    Path(room_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<(), Error> {
    // check if room exist and user has permission
    let rank = state.db.get_rank(claims.user_id, room_id).await?;
    if rank != RANK_OWNER {
        return Err(Error::Forbidden);
    }

    let cover = util::common::generate_cover_name(room_id);

    if let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(content_type) = field.content_type() {
            if !content_type.starts_with(IMAGE_KEY) {
                return Err(Error::BadRequest);
            }
        } else {
            return Err(Error::BadRequest);
        }

        let path = path::Path::new(&state.config.public_directory).join(&cover[1..]);
        let data = field.bytes().await.unwrap();
        tokio::fs::write(&path, &data).await?;

        let old_cover = state.db.change_cover(room_id, &cover).await?;
        if old_cover != PUBLIC_ROOM_COVER {
            let path = path::Path::new(&state.config.public_directory).join(&old_cover[1..]);
            if path.is_file() {
                tokio::fs::remove_file(&path).await?;
            }
        }
    } else {
        return Err(Error::BadRequest);
    }

    let rsp = ChangeCoverResponse { room_id, cover };
    let msg = ServerEvent::ChangeCover(rsp).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    Ok(())
}

pub async fn create_room(
    state: &Arc<AppState>,
    client: &Client,
    req: NewRoomRequest,
) -> Result<(), Error> {
    req.validate()?;

    // check if user is in the list
    if req.members_id.contains(&client.user_id()) {
        return Err(Error::Forbidden);
    }

    // create new room
    let room = state.db.create_room(client.user_id(), &req).await?;

    // let members join the room
    let room_id = room.id;
    let users_id: Vec<i64> = room.members.iter().map(|x| x.id).collect();
    state.hub.add_members(room_id, &users_id).await?;

    // send room info to members
    let rsp = NewRoomResponse { room };
    let msg = ServerEvent::NewRoom(rsp).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    Ok(())
}

pub async fn update_room(
    state: &Arc<AppState>,
    client: &Client,
    req: UpdateRoomResquest,
) -> Result<(), Error> {
    req.validate()?;

    // check if room exist and user has permission
    let rank = state.db.get_rank(client.user_id(), req.room_id).await?;
    if rank != RANK_OWNER {
        return Err(Error::Forbidden);
    }

    // update room name in database
    let rsp = state.db.update_room(&req).await?;

    // notice all room members
    let msg = ServerEvent::UpdateRoom(rsp).to_msg()?;
    state.hub.broadcast(req.room_id, msg).await?;

    Ok(())
}

pub async fn delete_room(
    state: &Arc<AppState>,
    client: &Client,
    req: DeleteRoomRequest,
) -> Result<(), Error> {
    req.validate()?;
    let room_id = req.room_id;

    // check if room exist and user has permission
    let rank = state.db.get_rank(client.user_id(), req.room_id).await?;
    if rank != RANK_OWNER {
        return Err(Error::Forbidden);
    }

    // delete room and room members in the database
    let members_id = state.db.delete_room(room_id).await?;

    // notice the room members
    let msg = ServerEvent::DeleteRoom(DeleteRoomResponse { room_id }).to_msg()?;
    state.hub.notify(&members_id, msg).await?;

    // remove room channels and subscriptions
    state.hub.delete_room(room_id, &members_id).await?;

    Ok(())
}

pub async fn leave_room(
    state: &Arc<AppState>,
    client: &Client,
    req: LeaveRoomRequest,
) -> Result<(), Error> {
    req.validate()?;

    // check if room exist and user has permission
    let user_id = client.user_id();
    let room_id = req.room_id;
    let rank = state.db.get_rank(user_id, room_id).await?;
    if rank == RANK_OWNER {
        return Err(Error::Forbidden);
    }

    // delete the room member in the database
    let req: DeleteMembersRequest = DeleteMembersRequest {
        room_id,
        members_id: vec![user_id],
    };
    let members_id = state.db.delete_members(&req).await?;

    // remove users from the room
    state.hub.remove_members(room_id, &members_id).await?;

    // notice user to delete room
    let rsp = DeleteRoomResponse { room_id };
    let msg = ServerEvent::DeleteRoom(rsp).to_msg()?;
    state.hub.broadcast(client.room_id(), msg).await?;

    // notice other room members to delete member
    let rsp = DeleteMembersResponse {
        room_id,
        members_id,
    };
    let msg = ServerEvent::DeleteMembers(rsp).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    Ok(())
}
