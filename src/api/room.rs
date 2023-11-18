//! Handlers for chat rooms

use super::DeleteMembersResponse;
use super::{
    event::ServerEvent, AppState, DeleteMembersRequest, DeleteRoomRequest, DeleteRoomResponse,
    LeaveRoomRequest, NewRoomRequest, NewRoomResponse, UpdateRoomResquest,
};
use crate::conn::Client;
use crate::core::{
    constant::{RANK_MEMBER, RANK_OWNER},
    Error,
};
use std::sync::Arc;
use validator::Validate;

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

    // check if room exist and user has permission (manager)
    let rank = state.db.get_rank(client.user_id(), req.room_id).await?;
    if rank == RANK_MEMBER {
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

    // check if room exist and user has permission (owner)
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
