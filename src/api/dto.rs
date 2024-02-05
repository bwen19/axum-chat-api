//! Data objects defined for HTTP request and response

use crate::{
    core::validator as VAL,
    store::{FriendInfo, MemberInfo, MessageInfo, RoomInfo, UserInfo},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use validator::Validate;

// ============================== // Auth // ============================== //

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(length(min = 2, max = 50, message = "Must be between 2 and 50 characters"))]
    pub username: String,
    #[validate(length(min = 6, max = 50, message = "Must be between 6 and 50 characters"))]
    pub password: String,
    pub is_admin: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expire_at: OffsetDateTime,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AutoLoginRequest {
    #[validate(length(min = 1, message = "Must not be blank"))]
    pub refresh_token: String,
    pub is_admin: Option<bool>,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RenewTokenRequest {
    #[validate(length(min = 1, message = "Must not be blank"))]
    pub refresh_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenewTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expire_at: OffsetDateTime,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    #[validate(length(min = 1, message = "Must not be blank"))]
    pub refresh_token: String,
}

// ============================== // User // ============================== //

#[derive(Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 2, max = 50, message = "Must be between 2 and 50 characters"))]
    pub username: String,
    #[validate(length(min = 6, max = 50, message = "Must be between 6 and 50 characters"))]
    pub password: String,
    #[validate(custom = "VAL::validate_user_role")]
    pub role: String,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub user: UserInfo,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    #[validate(range(min = 1, message = "user id is invalid"))]
    pub user_id: i64,
    #[validate(length(min = 2, max = 50, message = "Must be between 2 and 50 characters"))]
    pub username: Option<String>,
    #[validate(length(min = 6, max = 50, message = "Must be between 6 and 50 characters"))]
    pub password: Option<String>,
    #[validate(length(min = 2, max = 50, message = "Must be between 2 and 50 characters"))]
    pub nickname: Option<String>,
    #[validate(length(min = 1, max = 200, message = "Must be between 1 and 200 characters"))]
    pub avatar: Option<String>,
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
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[validate(length(min = 6, max = 50, message = "Must be between 6 and 50 characters"))]
    pub old_password: String,
    #[validate(length(min = 6, max = 50, message = "Must be between 6 and 50 characters"))]
    pub new_password: String,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersRequest {
    #[validate(range(min = 1, message = "Must be greater than 1"))]
    pub page_id: i64,
    #[validate(range(min = 5, max = 30, message = "Must be between 5 and 30"))]
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

// ============================== // Message // ============================== //

/// Used to get initial rooms and friends
#[derive(Deserialize)]
pub struct InitializeRequest {}

#[derive(Serialize)]
pub struct InitializeResponse {
    pub rooms: Vec<RoomInfo>,
    pub friends: Vec<FriendInfo>,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct NewMessageRequest {
    #[validate(range(min = 1, message = "Invalid ID"))]
    pub room_id: i64,
    #[validate(length(min = 1, max = 500, message = "Must be between 1 and 500 characters"))]
    pub content: String,
    pub file_url: String,
    #[validate(custom = "VAL::validate_message_kind")]
    pub kind: String,
}

#[derive(Serialize)]
pub struct NewMessageResponse {
    pub message: MessageInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFileResponse {
    pub content: String,
    pub file_url: String,
    pub kind: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubStatusResponse {
    pub num_users: usize,
    pub num_clients: usize,
    pub num_rooms: usize,
}

// ============================== // Room // ============================== //

#[derive(Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct NewRoomRequest {
    #[validate(length(min = 2, max = 50, message = "Must be between 2 and 50 characters"))]
    pub name: String,
    #[validate(
        length(min = 2, message = "Must have at least 2 members"),
        custom = "VAL::validate_id_vec"
    )]
    pub members_id: Vec<i64>,
}

#[derive(Serialize)]
pub struct NewRoomResponse {
    pub room: RoomInfo,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoomResquest {
    #[validate(range(min = 1, message = "Invalid room ID"))]
    pub room_id: i64,
    #[validate(length(min = 2, max = 50, message = "Must be between 2 and 50 characters"))]
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoomResponse {
    pub room_id: i64,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCoverResponse {
    pub room_id: i64,
    pub cover: String,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRoomRequest {
    #[validate(range(min = 1, message = "Invalid room ID"))]
    pub room_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRoomResponse {
    pub room_id: i64,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LeaveRoomRequest {
    #[validate(range(min = 1, message = "Invalid room ID"))]
    pub room_id: i64,
}

// ============================== // Member // ============================== //

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddMembersRequest {
    #[validate(range(min = 1, message = "Invalid ID"))]
    pub room_id: i64,
    #[validate(
        length(min = 1, message = "Must have at least 1 members"),
        custom = "VAL::validate_id_vec"
    )]
    pub members_id: Vec<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMembersResponse {
    pub room_id: i64,
    pub members: Vec<MemberInfo>,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMembersRequest {
    #[validate(range(min = 1, message = "Invalid ID"))]
    pub room_id: i64,
    #[validate(
        length(min = 1, message = "Must have at least 1 members"),
        custom = "VAL::validate_id_vec"
    )]
    pub members_id: Vec<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMembersResponse {
    pub room_id: i64,
    pub members_id: Vec<i64>,
}

// ============================== // Friend // ============================== //

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddFriendRequest {
    #[validate(range(min = 1, message = "Invalid user id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
pub struct AddFriendResponse {
    pub friend: FriendInfo,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct RefuseFriendRequest {
    #[validate(range(min = 1, message = "Invalid friend id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefuseFriendResponse {
    pub friend_id: i64,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFriendRequest {
    #[validate(range(min = 1, message = "Invalid friend id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFriendResponse {
    pub friend_id: i64,
    pub room_id: i64,
}
