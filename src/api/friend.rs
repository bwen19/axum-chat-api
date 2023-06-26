use crate::db::model::{FriendInfo, RoomInfo};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ========================// Friend //======================== //

// ---------------- Add friend ---------------- //
#[derive(Deserialize, Validate)]
pub struct AddFriendRequest {
    #[validate(range(min = 1, message = "Invalid friend id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
pub struct AddFriendResponse {
    pub friend: FriendInfo,
}

// ---------------- Accept friend ---------------- //
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

// ---------------- Refuse friend ---------------- //
#[derive(Deserialize, Validate)]
pub struct RefuseFriendRequest {
    #[validate(range(min = 1, message = "Invalid friend id"))]
    pub friend_id: i64,
}

#[derive(Serialize)]
pub struct RefuseFriendResponse {
    pub friend_id: i64,
}

// ---------------- Delete friend ---------------- //
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
