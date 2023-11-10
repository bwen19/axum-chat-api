use serde::Serialize;
use time::OffsetDateTime;

// ========================// User //======================== //

pub struct User {
    pub id: i64,
    pub username: String,
    pub hashed_password: String,
    pub avatar: String,
    pub nickname: String,
    pub role: String,
    pub room_id: i64,
    pub deleted: bool,
    pub create_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub avatar: String,
    pub nickname: String,
    pub role: String,
    pub room_id: i64,
    pub deleted: bool,
    pub create_at: OffsetDateTime,
}

impl From<User> for UserInfo {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            username: value.username,
            avatar: value.avatar,
            nickname: value.nickname,
            role: value.role,
            room_id: value.room_id,
            deleted: value.deleted,
            create_at: value.create_at,
        }
    }
}

// ========================// FriendShip //======================== //

pub struct Friend {
    pub requester_id: i64,
    pub addressee_id: i64,
    pub room_id: i64,
    pub status: String,
    pub create_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct FriendInfo {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
    pub status: String,
    pub room_id: i64,
    pub first: bool,
    pub create_at: OffsetDateTime,
}

// ========================// Room //======================== //

pub struct Room {
    pub id: i64,
    pub name: String,
    pub cover: String,
    pub category: String,
    pub create_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct RoomInfo {
    pub id: i64,
    pub name: String,
    pub cover: String,
    pub category: String,
    pub unreads: i64,
    pub create_at: OffsetDateTime,
    pub members: Vec<MemberInfo>,
    pub messages: Vec<MessageInfo>,
}

// ========================// Member //======================== //

#[derive(Serialize, Clone)]
pub struct MemberInfo {
    pub id: i64,
    pub name: String,
    pub avatar: String,
    pub rank: String,
    pub join_at: OffsetDateTime,
}

// ========================// Message //======================== //

#[derive(Serialize)]
pub struct MessageInfo {
    pub id: i64,
    pub room_id: i64,
    pub sender_id: i64,
    pub name: String,
    pub avatar: String,
    pub content: String,
    pub kind: String,
    pub divide: bool,
    pub send_at: OffsetDateTime,
}
