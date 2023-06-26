use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ========================// User //======================== //

pub struct User {
    pub id: i64,
    pub username: String,
    pub hashed_password: String,
    pub nickname: String,
    pub avatar: String,
    pub bio: String,
    pub role: String,
    pub deleted: bool,
    pub room_id: i64,
    pub create_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
    pub bio: String,
    pub role: String,
    pub deleted: bool,
    pub create_at: DateTime<Utc>,
}

impl From<User> for UserInfo {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            username: value.username,
            nickname: value.nickname,
            avatar: value.avatar,
            bio: value.bio,
            role: value.role,
            deleted: value.deleted,
            create_at: value.create_at,
        }
    }
}

// ========================// Session //======================== //

pub struct SessionEntity {
    pub id: Uuid,
    pub user_id: i64,
    pub refresh_token: String,
    pub client_ip: String,
    pub user_agent: String,
    pub expire_at: DateTime<Utc>,
    pub create_at: DateTime<Utc>,
}

// ========================// FriendShip //======================== //

pub struct FriendShip {
    pub user_id: i64,
    pub friend_id: i64,
    pub status: String,
    pub room_id: i64,
    pub create_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct FriendInfo {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
    pub bio: String,
    pub status: String,
    pub room_id: i64,
    pub first: bool,
    pub create_at: DateTime<Utc>,
}

// ========================// Room //======================== //

pub struct Room {
    pub id: i64,
    pub name: String,
    pub cover: String,
    pub category: String,
    pub create_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize)]
pub struct RoomInfo {
    pub id: i64,
    pub name: String,
    pub cover: String,
    pub category: String,
    pub unreads: i64,
    pub create_at: DateTime<Utc>,
    pub members: Vec<MemberInfo>,
    pub messages: Vec<MessageInfo>,
}

// ========================// Member //======================== //

#[derive(Deserialize, Serialize, Clone)]
pub struct MemberInfo {
    pub id: i64,
    pub name: String,
    pub avatar: String,
    pub rank: String,
    pub join_at: DateTime<Utc>,
}

// ========================// Message //======================== //

#[derive(Deserialize, Serialize)]
pub struct MessageInfo {
    pub id: i64,
    pub sid: i64,
    pub name: String,
    pub avatar: String,
    pub content: String,
    pub kind: String,
    pub divide: bool,
    pub send_at: DateTime<Utc>,
}

// ========================// Invitation //======================== //

#[derive(Deserialize, Serialize)]
pub struct Invitation {
    pub code: String,
    pub expire_at: DateTime<Utc>,
}
