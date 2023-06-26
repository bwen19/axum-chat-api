use super::event::{ClientEvent, ServerEvent};
use crate::api::friend::{
    AcceptFriendRequest, AcceptFriendResponse, AddFriendRequest, AddFriendResponse,
    DeleteFriendRequest, DeleteFriendResponse, RefuseFriendRequest, RefuseFriendResponse,
};
use crate::api::message::InitialRequest;
use crate::api::room::{
    DeleteRoomRequest, DeleteRoomResponse, LeaveRoomRequest, NewRoomRequest, UpdateRoomResquest,
};
use crate::api::{
    member::{AddMembersRequest, DeleteMembersRequest},
    message::{InitialResponse, NewMessageRequest},
};
use crate::{
    error::{AppError, AppResult},
    extractor::SocketGuard,
    AppState,
};
use axum::extract::ws::Message;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use validator::Validate;

// ========================// UserSocket //======================== //

#[derive(Clone)]
pub struct UserSocket {
    pub socket_id: Uuid,
    pub user_id: i64,
    // user's personal room
    pub room_id: i64,
    pub role: String,
    pub user_agent: String,
    // the mpsc channel used to pass message to user's socket
    pub tx: mpsc::Sender<Message>,
}

impl UserSocket {
    pub fn new(skg: SocketGuard, tx: mpsc::Sender<Message>) -> Self {
        let socket_id = Uuid::new_v4();
        Self {
            socket_id,
            user_id: skg.user_id,
            room_id: skg.room_id,
            role: skg.role,
            user_agent: skg.user_agent,
            tx,
        }
    }

    /// Handle events from Message::Text
    ///
    /// Return whether to continue receiving messages
    pub async fn handle_event(&self, state: &Arc<AppState>, event: ClientEvent) -> bool {
        // dispatch message to handlers
        let result = match event {
            ClientEvent::Pong(_) => return true,
            ClientEvent::Initialize(req) => self.initialize(state, req).await,
            ClientEvent::SendMessage(req) => self.send_message(state, req).await,
            ClientEvent::CreateRoom(req) => self.create_room(state, req).await,
            ClientEvent::UpdateRoom(req) => self.update_room(state, req).await,
            ClientEvent::DeleteRoom(req) => self.delete_room(state, req).await,
            ClientEvent::LeaveRoom(req) => self.leave_room(state, req).await,
            ClientEvent::AddMembers(req) => self.add_members(state, req).await,
            ClientEvent::DeleteMembers(req) => self.delete_members(state, req).await,
            ClientEvent::AddFriend(req) => self.add_friend(state, req).await,
            ClientEvent::AcceptFriend(req) => self.accept_friend(state, req).await,
            ClientEvent::RefuseFriend(req) => self.refuse_friend(state, req).await,
            ClientEvent::DeleteFriend(req) => self.delete_friend(state, req).await,
        };

        // send text message of WsError to the user's client
        if let Err(err) = result {
            match err {
                AppError::SendMessage => false,
                AppError::SerializeMessage => false,
                _ => {
                    let (_, msg) = err.into_error();
                    if let Ok(msg) = ServerEvent::ErrMessage(msg).to_msg() {
                        if self.tx.send(msg).await.is_err() {
                            false
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                }
            }
        } else {
            true
        }
    }

    async fn initialize(&self, state: &Arc<AppState>, req: InitialRequest) -> AppResult<()> {
        req.validate()?;

        // get room information from database
        let rooms = state.db.get_user_rooms(self.user_id, req.timestamp).await?;
        let friends = state.db.get_user_friends(self.user_id).await?;

        // create connections to the room channels
        let room_ids: Vec<i64> = rooms.iter().map(|room| room.id).collect();
        let capacity = state.config.room_channel_capacity;
        state.channel.connect(self, room_ids, capacity);

        // send rooms and friends info to the client socket
        let rsp = InitialResponse { rooms, friends };
        let msg = ServerEvent::Initialized(rsp).to_msg()?;
        self.tx.send(msg).await?;

        Ok(())
    }

    async fn send_message(&self, state: &Arc<AppState>, req: NewMessageRequest) -> AppResult<()> {
        req.validate()?;

        // check whether user is in the room
        if !state.channel.in_room(&self.socket_id, req.room_id) {
            return Err(AppError::NotInRoom);
        }

        // create message and store in database
        let data = state.db.create_message(self.user_id, &req).await?;

        // send message to the room
        let room_id = data.room_id;
        let msg = ServerEvent::ReceivedMessage(data).to_msg()?;
        state.channel.send_to_room(room_id, msg);

        Ok(())
    }

    async fn create_room(&self, state: &Arc<AppState>, req: NewRoomRequest) -> AppResult<()> {
        req.validate()?;

        // check whether current user in the new room
        if let Some(&id) = req.member_ids.get(0) {
            if id != self.user_id {
                return Err(AppError::NotInRoom);
            }
        }

        // create new room
        let rsp = state.db.create_room(&req).await?;

        // let members join the room
        let room_id = rsp.room.id;
        let user_ids: Vec<i64> = rsp.room.members.iter().map(|x| x.id).collect();
        let capacity = state.config.room_channel_capacity;
        state.channel.join_room_mult(room_id, &user_ids, capacity);

        // send room info to members
        let msg = ServerEvent::JoinedRoom(rsp).to_msg()?;
        state.channel.send_to_room(room_id, msg);

        Ok(())
    }

    async fn update_room(&self, state: &Arc<AppState>, req: UpdateRoomResquest) -> AppResult<()> {
        req.validate()?;

        // check if room exist and user has permission (manager)
        if !state
            .db
            .check_rank(self.user_id, req.room_id, "manager")
            .await?
        {
            return Err(AppError::Forbidden);
        }

        // update room name in database
        let rsp = state.db.update_room(&req).await?;

        // notice all room members
        let msg = ServerEvent::UpdatedRoom(rsp).to_msg()?;
        state.channel.send_to_room(req.room_id, msg);

        Ok(())
    }

    async fn delete_room(&self, state: &Arc<AppState>, req: DeleteRoomRequest) -> AppResult<()> {
        req.validate()?;
        let room_id = req.room_id;

        // check if room exist and user has permission (owner)
        if !state.db.check_rank(self.user_id, room_id, "owner").await? {
            return Err(AppError::Forbidden);
        }

        // delete room and room members in the database
        let member_ids = state.db.delete_room(room_id).await?;

        // notice the room members
        let msg = ServerEvent::DeletedRoom(DeleteRoomResponse { room_id }).to_msg()?;
        state.channel.send_to_users(&member_ids, msg);

        // remove room channels and subscriptions
        state.channel.leave_room_mult(room_id, member_ids);

        Ok(())
    }

    async fn leave_room(&self, state: &Arc<AppState>, req: LeaveRoomRequest) -> AppResult<()> {
        req.validate()?;
        let room_id = req.room_id;

        // delete the room member in the database
        let arg = DeleteMembersRequest {
            room_id,
            member_ids: vec![self.user_id],
        };
        let rsp = state.db.delete_members(&arg).await?;

        // remove room channels and subscriptions
        state.channel.leave_room(room_id, self.user_id);

        // notice all user's devices
        let msg = ServerEvent::DeletedRoom(DeleteRoomResponse { room_id }).to_msg()?;
        state.channel.send_to_room(self.room_id, msg);

        // notice room members
        let msg = ServerEvent::DeletedMembers(rsp).to_msg()?;
        state.channel.send_to_room(room_id, msg);

        Ok(())
    }

    async fn add_members(&self, state: &Arc<AppState>, req: AddMembersRequest) -> AppResult<()> {
        req.validate()?;

        // check if room exist and user has permission (manager)
        if !state
            .db
            .check_rank(self.user_id, req.room_id, "manager")
            .await?
        {
            return Err(AppError::Forbidden);
        }

        // insert members into database
        let rsp1 = state.db.add_members(&req).await?;
        let rsp2 = state.db.get_room(req.room_id).await?;
        let user_ids = rsp1.members.iter().map(|x| x.id).collect();

        // notice room members
        let msg = ServerEvent::AddedMembers(rsp1).to_msg()?;
        state.channel.send_to_room(req.room_id, msg);

        // notice the new members
        let capacity = state.config.room_channel_capacity;
        state
            .channel
            .join_room_mult(req.room_id, &user_ids, capacity);

        let msg = ServerEvent::JoinedRoom(rsp2).to_msg()?;
        state.channel.send_to_users(&user_ids, msg);

        Ok(())
    }

    async fn delete_members(
        &self,
        state: &Arc<AppState>,
        req: DeleteMembersRequest,
    ) -> AppResult<()> {
        req.validate()?;

        // check if room exist and user has permission (manager)
        if !state
            .db
            .check_rank(self.user_id, req.room_id, "manager")
            .await?
        {
            return Err(AppError::Forbidden);
        }

        // check if user is in the list
        if req.member_ids.contains(&self.user_id) {
            return Err(AppError::NotInRoom);
        }

        // delete members from database
        let rsp = state.db.delete_members(&req).await?;

        // remove connection in the channel
        state.channel.leave_room_mult(req.room_id, req.member_ids);

        // notice other room members
        let msg = ServerEvent::DeletedMembers(rsp).to_msg()?;
        state.channel.send_to_room(req.room_id, msg);

        Ok(())
    }

    async fn add_friend(&self, state: &Arc<AppState>, req: AddFriendRequest) -> AppResult<()> {
        req.validate()?;

        // check the friendship between users and adding friend
        let bi_friend = {
            if let Some(ship) = state.db.get_friend(self.user_id, req.friend_id).await? {
                if ship.status == "deleted" {
                    state.db.adding_friend(self.user_id, req.friend_id).await?
                } else {
                    return Err(AppError::NotInRoom);
                }
            } else {
                state.db.create_friend(self.user_id, req.friend_id).await?
            }
        };

        // send user info to the other side
        let rsp = AddFriendResponse {
            friend: bi_friend.ffriend,
        };
        let msg = ServerEvent::AddedFriend(rsp).to_msg()?;
        state.channel.send_to_user(req.friend_id, msg);

        // send the other's info to user self
        let rsp = AddFriendResponse {
            friend: bi_friend.ufriend,
        };
        let msg = ServerEvent::AddedFriend(rsp).to_msg()?;
        state.channel.send_to_room(self.room_id, msg);

        Ok(())
    }

    async fn accept_friend(
        &self,
        state: &Arc<AppState>,
        req: AcceptFriendRequest,
    ) -> AppResult<()> {
        req.validate()?;

        // check the friendship between users and adding friend
        let (friend, room) = state.db.accept_friend(req.friend_id, self.user_id).await?;

        // join in the new room
        let user_ids = vec![friend.ufriend.id, friend.ffriend.id];
        let capacity = state.config.room_channel_capacity;
        let room_id = room.uroom.id;
        state.channel.join_room_mult(room_id, &user_ids, capacity);

        // send user info to the other side
        let rsp = AcceptFriendResponse {
            friend: friend.ufriend,
            room: room.uroom,
        };
        let msg = ServerEvent::AcceptedFriend(rsp).to_msg()?;
        state.channel.send_to_user(req.friend_id, msg);

        // send the other's info to user self
        let rsp = AcceptFriendResponse {
            friend: friend.ffriend,
            room: room.froom,
        };
        let msg = ServerEvent::AcceptedFriend(rsp).to_msg()?;
        state.channel.send_to_room(self.room_id, msg);

        Ok(())
    }

    async fn refuse_friend(
        &self,
        state: &Arc<AppState>,
        req: RefuseFriendRequest,
    ) -> AppResult<()> {
        req.validate()?;

        // check the friendship between users and adding friend
        state.db.refuse_friend(req.friend_id, self.user_id).await?;

        // send user info to the other side
        let rsp = RefuseFriendResponse {
            friend_id: self.user_id,
        };
        let msg = ServerEvent::RefusedFriend(rsp).to_msg()?;
        state.channel.send_to_user(req.friend_id, msg);

        // send the other's info to user self
        let rsp = RefuseFriendResponse {
            friend_id: req.friend_id,
        };
        let msg = ServerEvent::RefusedFriend(rsp).to_msg()?;
        state.channel.send_to_room(self.room_id, msg);

        Ok(())
    }

    async fn delete_friend(
        &self,
        state: &Arc<AppState>,
        req: DeleteFriendRequest,
    ) -> AppResult<()> {
        req.validate()?;

        // check the friendship between users and adding friend
        let room_id = state.db.delete_friend(self.user_id, req.friend_id).await?;

        // disconnect room channel and send leave info to client
        let user_ids = vec![req.friend_id, self.user_id];
        state.channel.leave_room_mult(room_id, user_ids);

        // send user info to the other side
        let rsp = DeleteFriendResponse {
            friend_id: self.user_id,
            room_id,
        };
        let msg = ServerEvent::DeletedFriend(rsp).to_msg()?;
        state.channel.send_to_user(req.friend_id, msg);

        // send the other's info to user self
        let rsp = DeleteFriendResponse {
            friend_id: req.friend_id,
            room_id,
        };
        let msg = ServerEvent::DeletedFriend(rsp).to_msg()?;
        state.channel.send_to_room(self.room_id, msg);

        Ok(())
    }
}
