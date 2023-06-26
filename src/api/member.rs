use super::validator as VAL;
use crate::db::model::MemberInfo;
use serde::{Deserialize, Serialize};
use validator::Validate;

// ========================// Room Members //======================== //

// ---------------- Add members ---------------- //
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

// ---------------- Delete members ---------------- //
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
