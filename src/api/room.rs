//! Handlers for chat rooms

use super::{
    event::ServerEvent, AppState, DeleteMembersRequest, DeleteRoomRequest, DeleteRoomResponse,
    LeaveRoomRequest, NewRoomRequest, UpdateRoomResquest,
};
use crate::{conn::Client, core::Error};
use std::sync::Arc;
use validator::Validate;

pub async fn create_room(
    state: &Arc<AppState>,
    client: &Client,
    req: NewRoomRequest,
) -> Result<(), Error> {
    req.validate()?;

    // check whether current user in the new room
    if let Some(&id) = req.member_ids.get(0) {
        if id != client.user_id() {
            return Err(Error::NotInRoom);
        }
    }

    // create new room
    let rsp = state.db.create_room(&req).await?;

    // let members join the room
    let room_id = rsp.room.id;
    let user_ids: Vec<i64> = rsp.room.members.iter().map(|x| x.id).collect();
    state.hub.add_members(room_id, &user_ids).await?;

    // send room info to members
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
    if !state
        .db
        .check_rank(client.user_id(), req.room_id, "manager")
        .await?
    {
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
    if !state
        .db
        .check_rank(client.user_id(), room_id, "owner")
        .await?
    {
        return Err(Error::Forbidden);
    }

    // delete room and room members in the database
    let member_ids = state.db.delete_room(room_id).await?;

    // notice the room members
    let msg = ServerEvent::DeleteRoom(DeleteRoomResponse { room_id }).to_msg()?;
    state.hub.notify(&member_ids, msg).await?;

    // remove room channels and subscriptions
    state.hub.remove_members(room_id, &member_ids).await?;

    Ok(())
}

pub async fn leave_room(
    state: &Arc<AppState>,
    client: &Client,
    req: LeaveRoomRequest,
) -> Result<(), Error> {
    req.validate()?;
    let room_id = req.room_id;

    // delete the room member in the database
    let arg = DeleteMembersRequest {
        room_id,
        member_ids: vec![client.user_id()],
    };
    let rsp = state.db.delete_members(&arg).await?;

    // remove room channels and subscriptions
    state.hub.remove_member(room_id, client.user_id()).await?;

    // notice all user's devices
    let msg = ServerEvent::DeleteRoom(DeleteRoomResponse { room_id }).to_msg()?;
    state.hub.broadcast(client.room_id(), msg).await?;

    // notice room members
    let msg = ServerEvent::DeleteMembers(rsp).to_msg()?;
    state.hub.broadcast(room_id, msg).await?;

    Ok(())
}
