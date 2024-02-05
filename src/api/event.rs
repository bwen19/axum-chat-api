//! Event objects defined for WebSocket

use super::{friend, member, message, room, ChangeCoverResponse};
use super::{
    AcceptFriendRequest, AcceptFriendResponse, AddFriendRequest, AddFriendResponse,
    AddMembersRequest, AddMembersResponse, AppState, DeleteFriendRequest, DeleteFriendResponse,
    DeleteMembersRequest, DeleteMembersResponse, DeleteRoomRequest, DeleteRoomResponse,
    InitializeRequest, InitializeResponse, LeaveRoomRequest, NewMessageRequest, NewMessageResponse,
    NewRoomRequest, NewRoomResponse, RefuseFriendRequest, RefuseFriendResponse, UpdateRoomResponse,
    UpdateRoomResquest,
};
use crate::{conn::Client, core::Error};
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================== // ClientEvent // ============================== //

#[derive(Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ClientEvent {
    #[serde(rename = "initialize")]
    Initialize(InitializeRequest),

    #[serde(rename = "new-message")]
    NewMessage(NewMessageRequest),

    // Room
    #[serde(rename = "new-room")]
    NewRoom(NewRoomRequest),

    #[serde(rename = "update-room")]
    UpdateRoom(UpdateRoomResquest),

    #[serde(rename = "delete-room")]
    DeleteRoom(DeleteRoomRequest),

    #[serde(rename = "leave-room")]
    LeaveRoom(LeaveRoomRequest),

    // member
    #[serde(rename = "add-members")]
    AddMembers(AddMembersRequest),

    #[serde(rename = "delete-members")]
    DeleteMembers(DeleteMembersRequest),

    // Friend
    #[serde(rename = "add-friend")]
    AddFriend(AddFriendRequest),

    #[serde(rename = "accept-friend")]
    AcceptFriend(AcceptFriendRequest),

    #[serde(rename = "refuse-friend")]
    RefuseFriend(RefuseFriendRequest),

    #[serde(rename = "delete-friend")]
    DeleteFriend(DeleteFriendRequest),
}

impl ClientEvent {
    pub async fn process(self, state: &Arc<AppState>, client: &Client) -> Result<(), Error> {
        let result = match self {
            ClientEvent::Initialize(_) => message::initialize(state, client).await,
            ClientEvent::NewMessage(req) => message::send_message(state, client, req).await,
            ClientEvent::NewRoom(req) => room::create_room(state, client, req).await,
            ClientEvent::UpdateRoom(req) => room::update_room(state, client, req).await,
            ClientEvent::DeleteRoom(req) => room::delete_room(state, client, req).await,
            ClientEvent::LeaveRoom(req) => room::leave_room(state, client, req).await,
            ClientEvent::AddMembers(req) => member::add_members(state, client, req).await,
            ClientEvent::DeleteMembers(req) => member::delete_members(state, client, req).await,
            ClientEvent::AddFriend(req) => friend::add_friend(state, client, req).await,
            ClientEvent::AcceptFriend(req) => friend::accept_friend(state, client, req).await,
            ClientEvent::RefuseFriend(req) => friend::refuse_friend(state, client, req).await,
            ClientEvent::DeleteFriend(req) => friend::delete_friend(state, client, req).await,
        };

        // send text message of WsError to the user's client
        if let Err(err) = result {
            match err {
                Error::SendMessage => return Err(err),
                _ => {
                    let (_, msg) = err.into_error();
                    let msg = ServerEvent::ErrMessage(msg).to_msg()?;
                    client.send(msg).await?;
                }
            }
        }
        Ok(())
    }
}

// ============================== // ServerEvent // ============================== //

#[derive(Serialize)]
#[serde(tag = "action", content = "data")]
pub enum ServerEvent {
    #[serde(rename = "toast")]
    ErrMessage(String),

    #[serde(rename = "initialize")]
    Initialize(InitializeResponse),

    #[serde(rename = "new-message")]
    NewMessage(NewMessageResponse),

    // Room
    #[serde(rename = "new-room")]
    NewRoom(NewRoomResponse),

    #[serde(rename = "change-cover")]
    ChangeCover(ChangeCoverResponse),

    #[serde(rename = "update-room")]
    UpdateRoom(UpdateRoomResponse),

    #[serde(rename = "delete-room")]
    DeleteRoom(DeleteRoomResponse),

    // Member
    #[serde(rename = "add-members")]
    AddMembers(AddMembersResponse),

    #[serde(rename = "delete-members")]
    DeleteMembers(DeleteMembersResponse),

    // Friend
    #[serde(rename = "add-friend")]
    AddFriend(AddFriendResponse),

    #[serde(rename = "accept-friend")]
    AcceptFriend(AcceptFriendResponse),

    #[serde(rename = "refuse-friend")]
    RefuseFriend(RefuseFriendResponse),

    #[serde(rename = "delete-friend")]
    DeleteFriend(DeleteFriendResponse),
}

impl ServerEvent {
    pub fn to_msg(&self) -> Result<Message, Error> {
        let msg = serde_json::to_string(self)?;
        Ok(Message::Text(msg))
    }
}
