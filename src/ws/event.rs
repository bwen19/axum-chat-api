use crate::{
    api::{
        friend::{
            AcceptFriendRequest, AcceptFriendResponse, AddFriendRequest, AddFriendResponse,
            DeleteFriendRequest, DeleteFriendResponse, RefuseFriendRequest, RefuseFriendResponse,
        },
        member::{
            AddMembersRequest, AddMembersResponse, DeleteMembersRequest, DeleteMembersResponse,
        },
        message::{InitialRequest, InitialResponse, NewMessageRequest, NewMessageResponse},
        room::{
            DeleteRoomRequest, DeleteRoomResponse, LeaveRoomRequest, NewRoomRequest,
            NewRoomResponse, UpdateRoomResponse, UpdateRoomResquest,
        },
    },
    error::{AppError, AppResult},
};
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};

// ========================// ClientEvent //======================== //

/// Events from client to server
#[derive(Deserialize)]
pub enum ClientEvent {
    Pong(u16),
    Initialize(InitialRequest),
    SendMessage(NewMessageRequest),
    // Room
    CreateRoom(NewRoomRequest),
    UpdateRoom(UpdateRoomResquest),
    DeleteRoom(DeleteRoomRequest),
    LeaveRoom(LeaveRoomRequest),
    // member
    AddMembers(AddMembersRequest),
    DeleteMembers(DeleteMembersRequest),
    // Friend
    AddFriend(AddFriendRequest),
    AcceptFriend(AcceptFriendRequest),
    RefuseFriend(RefuseFriendRequest),
    DeleteFriend(DeleteFriendRequest),
}

// ========================// ServerEvent //======================== //

/// Events from server to client
#[derive(Serialize)]
pub enum ServerEvent {
    Ping(u16),
    ErrMessage(String),
    Initialized(InitialResponse),
    ReceivedMessage(NewMessageResponse),
    // Room
    JoinedRoom(NewRoomResponse),
    UpdatedRoom(UpdateRoomResponse),
    DeletedRoom(DeleteRoomResponse),
    // Member
    AddedMembers(AddMembersResponse),
    DeletedMembers(DeleteMembersResponse),
    // Friend
    AddedFriend(AddFriendResponse),
    AcceptedFriend(AcceptFriendResponse),
    RefusedFriend(RefuseFriendResponse),
    DeletedFriend(DeleteFriendResponse),
}

impl ServerEvent {
    pub fn to_msg(&self) -> AppResult<Message> {
        serde_json::to_string(self)
            .map(|data| Message::Text(data))
            .map_err(|_| AppError::SerializeMessage)
    }
}
