use super::validator as VAL;
use crate::db::model::RoomInfo;
use serde::{Deserialize, Serialize};
use validator::Validate;

// ========================// Room //======================== //

// ---------------- New room ---------------- //
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

#[derive(Deserialize, Serialize)]
pub struct NewRoomResponse {
    pub room: RoomInfo,
}

// ---------------- Update room ---------------- //
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

// ---------------- Delete room ---------------- //
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
