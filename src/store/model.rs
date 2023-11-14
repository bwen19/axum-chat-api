use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use time::OffsetDateTime;

// ========================= // User // ========================= //

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

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub avatar: String,
    pub nickname: String,
    pub role: String,
    pub room_id: i64,
    pub deleted: bool,
    #[serde(with = "time::serde::rfc3339")]
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

// ========================= // FriendShip // ========================= //

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
    #[serde(with = "time::serde::rfc3339")]
    pub create_at: OffsetDateTime,
}

// ========================= // Room // ========================= //

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
    #[serde(with = "time::serde::rfc3339")]
    pub create_at: OffsetDateTime,
    pub members: Vec<MemberInfo>,
    pub messages: Vec<MessageInfo>,
}

impl From<(Room, Vec<MemberInfo>)> for RoomInfo {
    fn from((r, m): (Room, Vec<MemberInfo>)) -> Self {
        Self {
            id: r.id,
            name: r.name,
            cover: r.cover,
            category: r.category,
            unreads: 0,
            create_at: r.create_at,
            members: m,
            messages: Vec::new(),
        }
    }
}

/// compare two rooms by the latest message
pub fn cmp_room(a: &RoomInfo, b: &RoomInfo) -> Ordering {
    let a = a.messages.last();
    let b = b.messages.last();

    if let Some(a) = a {
        if let Some(b) = b {
            b.send_at.cmp(&a.send_at)
        } else {
            Ordering::Less
        }
    } else {
        Ordering::Greater
    }
}

// ========================= // Member // ========================= //

#[derive(Serialize, Clone)]
pub struct MemberInfo {
    pub id: i64,
    pub name: String,
    pub avatar: String,
    pub rank: String,
    #[serde(with = "time::serde::rfc3339")]
    pub join_at: OffsetDateTime,
}

fn numeric_rank(rank: &str) -> i64 {
    match rank {
        "owner" => 1,
        "manager" => 2,
        "member" => 5,
        _ => 10,
    }
}

pub fn cmp_member(a: &MemberInfo, b: &MemberInfo) -> Ordering {
    let a_rank = numeric_rank(&a.rank);
    let b_rank = numeric_rank(&b.rank);
    a_rank.cmp(&b_rank)
}

// ========================= // Message // ========================= //

#[derive(Serialize, Deserialize)]
pub struct MessageInfo {
    pub room_id: i64,
    pub sender_id: i64,
    pub name: String,
    pub avatar: String,
    pub content: String,
    pub kind: String,
    pub divide: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub send_at: OffsetDateTime,
}
