//! Handlers for room members

use super::{event::ServerEvent, AddMembersRequest, AppState, DeleteMembersRequest};
use crate::{conn::Client, core::Error};
use std::sync::Arc;
use validator::Validate;

// ========================// Room Members //======================== //

pub async fn add_members(
    state: &Arc<AppState>,
    client: &Client,
    req: AddMembersRequest,
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

    // insert members into database
    let rsp1 = state.db.add_members(&req).await?;
    let rsp2 = state.db.get_room(req.room_id).await?;
    let user_ids = rsp1.members.iter().map(|x| x.id).collect();

    // notice room members
    let msg = ServerEvent::AddMembers(rsp1).to_msg()?;
    state.hub.broadcast(req.room_id, msg).await?;

    // notice the new members
    state.hub.add_members(req.room_id, &user_ids).await?;

    let msg = ServerEvent::NewRoom(rsp2).to_msg()?;
    state.hub.notify(&user_ids, msg).await?;

    Ok(())
}

pub async fn delete_members(
    state: &Arc<AppState>,
    client: &Client,
    req: DeleteMembersRequest,
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

    // check if user is in the list
    if req.member_ids.contains(&client.user_id()) {
        return Err(Error::NotInRoom);
    }

    // delete members from database
    let rsp = state.db.delete_members(&req).await?;

    // remove connection in the hub
    state.hub.remove_members(req.room_id, &req.member_ids).await?;

    // notice other room members
    let msg = ServerEvent::DeleteMembers(rsp).to_msg()?;
    state.hub.broadcast(req.room_id, msg).await?;

    Ok(())
}
