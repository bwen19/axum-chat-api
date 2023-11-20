//! Handlers for room members

use super::{
    event::ServerEvent, AddMembersRequest, AddMembersResponse, AppState, DeleteMembersRequest,
    DeleteMembersResponse, DeleteRoomResponse, NewRoomResponse,
};
use crate::{
    conn::Client,
    core::{constant::RANK_OWNER, Error},
};
use std::sync::Arc;
use validator::Validate;

pub async fn add_members(
    state: &Arc<AppState>,
    client: &Client,
    req: AddMembersRequest,
) -> Result<(), Error> {
    req.validate()?;

    // check if room exist and user has permission
    let room_id = req.room_id;
    let rank = state.db.get_rank(client.user_id(), room_id).await?;
    if rank != RANK_OWNER {
        return Err(Error::Forbidden);
    }

    // insert members into database
    let members = state.db.add_members(&req).await?;
    let users_id = members.iter().map(|x| x.id).collect();

    // notice room members
    let rsp = AddMembersResponse { room_id, members };
    let msg = ServerEvent::AddMembers(rsp).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    // join members in the chat room
    state.hub.add_members(room_id, &users_id).await?;

    // notice the new members
    let room = state.db.get_room(room_id).await?;
    let rsp = NewRoomResponse { room };
    let msg = ServerEvent::NewRoom(rsp).to_msg()?;
    state.hub.notify(&users_id, msg).await?;

    Ok(())
}

pub async fn delete_members(
    state: &Arc<AppState>,
    client: &Client,
    req: DeleteMembersRequest,
) -> Result<(), Error> {
    req.validate()?;

    // check if room exist and user has permission
    let room_id = req.room_id;
    let rank = state.db.get_rank(client.user_id(), room_id).await?;
    if rank != RANK_OWNER {
        return Err(Error::Forbidden);
    }

    // check if user is in the list
    if req.members_id.contains(&client.user_id()) {
        return Err(Error::Forbidden);
    }

    // delete members from database
    let members_id = state.db.delete_members(&req).await?;

    // remove connection in the hub
    state.hub.remove_members(room_id, &members_id).await?;

    // notice the deleted members
    let rsp = DeleteRoomResponse { room_id };
    let msg = ServerEvent::DeleteRoom(rsp).to_msg()?;
    state.hub.notify(&members_id, msg).await?;

    // notice other room members
    let rsp = DeleteMembersResponse {
        room_id,
        members_id,
    };
    let msg = ServerEvent::DeleteMembers(rsp).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    Ok(())
}
