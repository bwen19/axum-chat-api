//! Data objects defined for HTTP request and response

use crate::{
    core::validator as VAL,
    store::{FriendInfo, MemberInfo, MessageInfo, RoomInfo, UserInfo},
};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ============================== // Auth // ============================== //

#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub username: String,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub password: String,
    pub is_admin: Option<bool>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct AutoLoginRequest {
    pub is_admin: Option<bool>,
}

#[derive(Serialize)]
pub struct RenewTokenResponse {
    pub access_token: String,
}

// ============================== // User // ============================== //

#[derive(Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub username: String,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub password: String,
    #[validate(custom = "VAL::validate_user_role")]
    pub role: String,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub user: UserInfo,
}

#[derive(Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(range(min = 1, message = "user id is invalid"))]
    pub user_id: i64,
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub username: Option<String>,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub password: Option<String>,
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub nickname: Option<String>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub avatar: Option<String>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub bio: Option<String>,
    #[validate(custom = "VAL::validate_user_role")]
    pub role: Option<String>,
    pub deleted: Option<bool>,
}

#[derive(Serialize)]
pub struct UpdateUserResponse {
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct ChangeAvatarResponse {
    pub avatar: String,
}

#[derive(Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub old_password: String,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub new_password: String,
}

#[derive(Deserialize, Validate)]
pub struct ListUsersRequest {
    #[validate(range(min = 1, message = "must be greater than 1"))]
    pub page_id: i64,
    #[validate(range(min = 5, max = 30, message = "must be between 5 and 30"))]
    pub page_size: i64,
}

#[derive(Serialize)]
pub struct ListUsersResponse {
    pub total: i64,
    pub users: Vec<UserInfo>,
}

#[derive(Serialize)]
pub struct FindUserResponse {
    pub user: Option<UserInfo>,
}

// ============================== // Friend // ============================== //

#[derive(Deserialize, Validate)]
pub struct AddFriendRequest {
    #[validate(range(min = 1, message = "Invalid user id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
pub struct AddFriendResponse {
    pub friend: FriendInfo,
}

#[derive(Deserialize, Validate)]
pub struct AcceptFriendRequest {
    #[validate(range(min = 1, message = "Invalid friend id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
pub struct AcceptFriendResponse {
    pub friend: FriendInfo,
    pub room: RoomInfo,
}

#[derive(Deserialize, Validate)]
pub struct RefuseFriendRequest {
    #[validate(range(min = 1, message = "Invalid friend id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
pub struct RefuseFriendResponse {
    pub friend_id: i64,
}

#[derive(Deserialize, Validate)]
pub struct DeleteFriendRequest {
    #[validate(range(min = 1, message = "Invalid friend id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
pub struct DeleteFriendResponse {
    pub friend_id: i64,
    pub room_id: i64,
}

// ============================== // Message // ============================== //

/// Used to get initial rooms and friends
#[derive(Deserialize, Validate)]
pub struct InitializeRequest {
    #[validate(range(min = 1, message = "invalid timestamp"))]
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct InitializeResponse {
    pub rooms: Vec<RoomInfo>,
    pub friends: Vec<FriendInfo>,
}

#[derive(Deserialize, Validate)]
pub struct NewMessageRequest {
    #[validate(range(min = 1, message = "invalid ID"))]
    pub room_id: i64,
    #[validate(length(min = 1, max = 500, message = "must be between 1 and 500 characters"))]
    pub content: String,
    #[validate(custom = "VAL::validate_message_kind")]
    pub kind: String,
}

#[derive(Serialize)]
pub struct NewMessageResponse {
    pub room_id: i64,
    pub message: MessageInfo,
}

#[derive(Serialize)]
pub struct SendFileResponse {
    pub file_url: String,
}

// ============================== // Room // ============================== //

#[derive(Deserialize, Serialize, Validate)]
pub struct NewRoomRequest {
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub name: String,
    #[validate(
        length(min = 3, message = "must have at least 3 members"),
        custom = "VAL::validate_id_vec"
    )]
    pub member_ids: Vec<i64>,
}

#[derive(Serialize)]
pub struct NewRoomResponse {
    pub room: RoomInfo,
}

#[derive(Deserialize, Validate)]
pub struct UpdateRoomResquest {
    #[validate(range(min = 1, message = "invalid room ID"))]
    pub room_id: i64,
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub name: String,
}

#[derive(Serialize)]
pub struct UpdateRoomResponse {
    pub room_id: i64,
    pub name: String,
}

#[derive(Deserialize, Validate)]
pub struct DeleteRoomRequest {
    #[validate(range(min = 1, message = "invalid room ID"))]
    pub room_id: i64,
}

#[derive(Serialize)]
pub struct DeleteRoomResponse {
    pub room_id: i64,
}

#[derive(Deserialize, Validate)]
pub struct LeaveRoomRequest {
    #[validate(range(min = 1, message = "invalid room ID"))]
    pub room_id: i64,
}

// ============================== // Member // ============================== //

#[derive(Deserialize, Validate)]
pub struct AddMembersRequest {
    #[validate(range(min = 1, message = "invalid ID"))]
    pub room_id: i64,
    #[validate(
        length(min = 1, message = "must have at least 1 members"),
        custom = "VAL::validate_id_vec"
    )]
    pub member_ids: Vec<i64>,
}

#[derive(Serialize)]
pub struct AddMembersResponse {
    pub room_id: i64,
    pub members: Vec<MemberInfo>,
}

#[derive(Deserialize, Validate)]
pub struct DeleteMembersRequest {
    #[validate(range(min = 1, message = "invalid ID"))]
    pub room_id: i64,
    #[validate(
        length(min = 1, message = "must have at least 1 members"),
        custom = "VAL::validate_id_vec"
    )]
    pub member_ids: Vec<i64>,
}

#[derive(Serialize)]
pub struct DeleteMembersResponse {
    pub room_id: i64,
    pub member_ids: Vec<i64>,
}