//! Handlers for friendship

use super::{
    event::ServerEvent, AcceptFriendRequest, AcceptFriendResponse, AddFriendRequest,
    AddFriendResponse, AppState, DeleteFriendRequest, DeleteFriendResponse, RefuseFriendRequest,
    RefuseFriendResponse,
};
use crate::{
    conn::Client,
    core::{constant::STATUS_DELETED, Error},
};
use std::sync::Arc;
use validator::Validate;

pub async fn add_friend(
    state: &Arc<AppState>,
    client: &Client,
    req: AddFriendRequest,
) -> Result<(), Error> {
    req.validate()?;

    // get friendship and create one if not exists
    let friend = {
        if let Some(friend) = state.db.get_friend(client.user_id(), req.friend_id).await? {
            if friend.status == STATUS_DELETED {
                state
                    .db
                    .update_friend(client.user_id(), req.friend_id)
                    .await?
            } else {
                return Err(Error::FriendStatus);
            }
        } else {
            state
                .db
                .create_friend(client.user_id(), req.friend_id)
                .await?
        }
    };

    let (requester, addressee) = state.db.get_friend_info(&friend).await?;

    // send requester info to the friend client
    let rsp = AddFriendResponse { friend: requester };
    let msg = ServerEvent::AddFriend(rsp).to_msg()?;
    state.hub.tell(req.friend_id, msg).await?;

    // send addressee info to user client
    let rsp = AddFriendResponse { friend: addressee };
    let msg = ServerEvent::AddFriend(rsp).to_msg()?;
    state.hub.broadcast(client.room_id(), msg).await?;

    Ok(())
}

pub async fn accept_friend(
    state: &Arc<AppState>,
    client: &Client,
    req: AcceptFriendRequest,
) -> Result<(), Error> {
    req.validate()?;

    let friend = state
        .db
        .get_friend(client.user_id(), req.friend_id)
        .await?
        .ok_or(Error::NotFound)?;

    // only addressee can accept the friendship
    if friend.addressee_id != client.user_id() {
        return Err(Error::FriendStatus);
    }

    // update friend status and add friend to the private room
    state.db.accept_friend(&friend).await?;

    let (requester, addressee) = state.db.get_friend_info(&friend).await?;
    let (room0, room1) = state.db.get_friend_room(&friend).await?;

    // join friends in the hub room
    let user_ids = vec![friend.requester_id, friend.addressee_id];
    state.hub.add_members(friend.room_id, &user_ids).await?;

    // send requester info to the user side
    let rsp = AcceptFriendResponse {
        friend: requester,
        room: room0,
    };
    let msg = ServerEvent::AcceptFriend(rsp).to_msg()?;
    state.hub.broadcast(client.room_id(), msg).await?;

    // send the addressee info to friend client
    let rsp = AcceptFriendResponse {
        friend: addressee,
        room: room1,
    };
    let msg = ServerEvent::AcceptFriend(rsp).to_msg()?;
    state.hub.tell(req.friend_id, msg).await?;

    Ok(())
}

pub async fn refuse_friend(
    state: &Arc<AppState>,
    client: &Client,
    req: RefuseFriendRequest,
) -> Result<(), Error> {
    req.validate()?;

    let friend = state
        .db
        .get_friend(client.user_id(), req.friend_id)
        .await?
        .ok_or(Error::NotFound)?;

    // update friend status in database
    state.db.refuse_friend(&friend).await?;

    // send user's info to the friend side
    let rsp = RefuseFriendResponse {
        friend_id: client.user_id(),
    };
    let msg = ServerEvent::RefuseFriend(rsp).to_msg()?;
    state.hub.tell(req.friend_id, msg).await?;

    // send the friend's info to user client
    let rsp = RefuseFriendResponse {
        friend_id: req.friend_id,
    };
    let msg = ServerEvent::RefuseFriend(rsp).to_msg()?;
    state.hub.broadcast(client.room_id(), msg).await?;

    Ok(())
}

pub async fn delete_friend(
    state: &Arc<AppState>,
    client: &Client,
    req: DeleteFriendRequest,
) -> Result<(), Error> {
    req.validate()?;

    let friend = state
        .db
        .get_friend(client.user_id(), req.friend_id)
        .await?
        .ok_or(Error::NotFound)?;

    // update friend status and remove users from the private chat room
    state.db.delete_friend(&friend).await?;

    // delete the chat room in the hub
    let users = vec![friend.requester_id, friend.addressee_id];
    state.hub.delete_room(friend.room_id, &users).await?;

    // send user's info to the friend room
    let rsp = DeleteFriendResponse {
        friend_id: client.user_id(),
        room_id: friend.room_id,
    };
    let msg = ServerEvent::DeleteFriend(rsp).to_msg()?;
    state.hub.tell(req.friend_id, msg).await?;

    // send the friend's info to user client
    let rsp = DeleteFriendResponse {
        friend_id: req.friend_id,
        room_id: friend.room_id,
    };
    let msg = ServerEvent::DeleteFriend(rsp).to_msg()?;
    state.hub.broadcast(client.room_id(), msg).await?;

    Ok(())
}
