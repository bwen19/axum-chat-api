use super::model::{MessageInfo, RoomInfo};
use super::{model::Room, Store};
use crate::api::room::{NewRoomRequest, NewRoomResponse, UpdateRoomResponse, UpdateRoomResquest};
use crate::db::model::MemberInfo;
use crate::error::{AppError, AppResult, ResultExt};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::collections::{hash_map::Entry, HashMap};

// ========================// Room Store //======================== //

impl Store {
    /// Create room and add initial members
    ///
    /// The first member will be the owner of the room.
    pub async fn create_room(&self, arg: &NewRoomRequest) -> AppResult<NewRoomResponse> {
        let mut transaction = self.pool.begin().await?;

        // create new room
        let room = sqlx::query_as!(
            Room,
            r#"
                INSERT INTO rooms (name, cover)
                VALUES ($1, '/cover/public')
                RETURNING id, name, cover, category, create_at
            "#,
            arg.name,
        )
        .fetch_one(&mut *transaction)
        .await
        .exactly_one()?;

        // make input of room members
        let room_ids = vec![room.id; arg.member_ids.len()];
        let mut ranks = vec!["user".to_string(); arg.member_ids.len()];
        if let Some(rank) = ranks.first_mut() {
            *rank = "owner".into();
        }

        // insert members and get members info
        let members = sqlx::query_as!(
            MemberInfo,
            r#"
                WITH insert_cte AS (
                    INSERT INTO room_members (room_id, member_id, rank)
                    SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::varchar[])
                    RETURNING member_id, rank, join_at
                )
                SELECT u.id, u.nickname AS name, u.avatar, y.rank, y.join_at
                FROM insert_cte AS y
                INNER JOIN users AS u ON y.member_id = u.id
            "#,
            &room_ids,
            &arg.member_ids,
            &ranks,
        )
        .fetch_all(&mut *transaction)
        .await
        .map(|data| data.into_iter().map(|v| v.into()).collect())?;

        transaction.commit().await?;

        Ok(collect_new_room(room, members))
    }

    /// Get room info when join a new room
    pub async fn get_room(&self, room_id: i64) -> AppResult<NewRoomResponse> {
        let room = sqlx::query_as!(
            Room,
            r#"
                SELECT id, name, cover, category, create_at
                FROM rooms
                WHERE id = $1
            "#,
            room_id,
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()?;

        // insert members and get members info
        let members = sqlx::query_as!(
            MemberInfo,
            r#"
                SELECT u.id, u.nickname AS name, u.avatar, y.rank, y.join_at
                FROM room_members AS y
                INNER JOIN users AS u ON y.member_id = u.id
                WHERE y.room_id = $1
            "#,
            room_id,
        )
        .fetch_all(&self.pool)
        .await
        .map(|data| data.into_iter().map(|v| v.into()).collect())?;

        Ok(collect_new_room(room, members))
    }

    /// Delete the room and return the member's id
    ///
    /// In general, only the admin and owner of the room can delete the room.
    /// So, one should check that before call the function.
    pub async fn delete_room(&self, room_id: i64) -> AppResult<Vec<i64>> {
        // get all members ID
        let member_ids = sqlx::query_scalar!(
            r#"
                SELECT member_id
                FROM room_members
                WHERE room_id = $1
            "#,
            room_id
        )
        .fetch_all(&self.pool)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM rooms
                WHERE id = $1
            "#,
            room_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(member_ids)
    }

    pub async fn update_room(&self, arg: &UpdateRoomResquest) -> AppResult<UpdateRoomResponse> {
        sqlx::query_as!(
            UpdateRoomResponse,
            r#"
                UPDATE rooms
                SET name = $1
                WHERE id = $2
                RETURNING id AS room_id, name
            "#,
            arg.name,
            arg.room_id,
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()
    }

    /// Get a list of rooms that the user has joined
    pub async fn get_user_rooms(&self, user_id: i64, timestamp: i64) -> AppResult<Vec<RoomInfo>> {
        // get user's rooms and members of each room
        let room_members = sqlx::query_as!(
            RoomMemberRow,
            r#"
                WITH rooms_cte AS (
                    SELECT id AS room_id, name, cover, category, create_at
                    FROM rooms WHERE id IN (
                        SELECT room_id FROM room_members
                        WHERE member_id = $1)
                )
                SELECT room_id, name, cover, category, create_at,
                    member_id, rank, join_at, nickname, avatar
                FROM rooms_cte AS r,
                    LATERAL (
                        SELECT member_id, rank, join_at, nickname, avatar
                        FROM room_members AS y
                        INNER JOIN users AS u ON y.member_id = u.id
                        WHERE y.room_id = r.room_id
                    ) AS m
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        let room_messages = sqlx::query_as!(
            RoomMessageRow,
            r#"
                SELECT room_id, message_id, sender_id, sender_nickname,
                    sender_avatar, content, kind, send_at
                FROM (
                    SELECT room_id, join_at
                    FROM room_members
                    WHERE member_id = $1
                ) AS r,
                    LATERAL (
                        SELECT m.id AS message_id, sender_id, u.nickname AS sender_nickname,
                            u.avatar AS sender_avatar, content, kind, m.send_at
                        FROM messages AS m
                        INNER JOIN users AS u ON u.id = m.sender_id
                        WHERE m.room_id = r.room_id AND m.send_at > r.join_at
                        ORDER BY m.send_at DESC
                        LIMIT 16
                    ) AS m
                ORDER BY r.room_id, m.send_at
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        collect_user_rooms(room_members, room_messages, user_id, timestamp)
    }
}

// ========================// Conversions //======================== //

fn collect_new_room(room: Room, members: Vec<MemberInfo>) -> NewRoomResponse {
    NewRoomResponse {
        room: RoomInfo {
            id: room.id,
            name: room.name,
            cover: room.cover,
            category: room.category,
            create_at: room.create_at,
            unreads: 0,
            members,
            messages: Vec::new(),
        },
    }
}

struct RoomMemberRow {
    room_id: i64,
    name: String,
    cover: String,
    category: String,
    create_at: DateTime<Utc>,
    member_id: i64,
    nickname: String,
    avatar: String,
    rank: String,
    join_at: DateTime<Utc>,
}

struct RoomMessageRow {
    room_id: i64,
    message_id: i64,
    sender_id: i64,
    sender_nickname: String,
    sender_avatar: String,
    content: String,
    kind: String,
    send_at: DateTime<Utc>,
}

impl From<RoomMessageRow> for MessageInfo {
    fn from(v: RoomMessageRow) -> Self {
        Self {
            id: v.message_id,
            sid: v.sender_id,
            name: v.sender_nickname,
            avatar: v.sender_avatar,
            content: v.content,
            kind: v.kind,
            divide: false,
            send_at: v.send_at,
        }
    }
}

fn collect_user_rooms(
    room_members: Vec<RoomMemberRow>,
    room_messages: Vec<RoomMessageRow>,
    user_id: i64,
    timestamp: i64,
) -> AppResult<Vec<RoomInfo>> {
    let mut rooms_map: HashMap<i64, RoomInfo> = HashMap::new();

    for item in room_members {
        let member = MemberInfo {
            id: item.member_id,
            name: item.nickname,
            avatar: item.avatar,
            rank: item.rank,
            join_at: item.join_at,
        };

        let is_private = item.category == "private" && member.id != user_id;

        match rooms_map.entry(item.room_id) {
            Entry::Occupied(mut o) => {
                let room = o.get_mut();
                if is_private {
                    room.name = member.name.clone();
                    room.cover = member.avatar.clone();
                }
                room.members.push(member);
            }
            Entry::Vacant(v) => {
                let room = if is_private {
                    RoomInfo {
                        id: item.room_id,
                        name: member.name.clone(),
                        cover: member.avatar.clone(),
                        category: item.category,
                        create_at: item.create_at,
                        unreads: 0,
                        members: vec![member],
                        messages: Vec::new(),
                    }
                } else {
                    RoomInfo {
                        id: item.room_id,
                        name: item.name,
                        cover: item.cover,
                        category: item.category,
                        create_at: item.create_at,
                        unreads: 0,
                        members: vec![member],
                        messages: Vec::new(),
                    }
                };
                v.insert(room);
            }
        }
    }

    let mut offset = 0_i64;
    for item in room_messages {
        match rooms_map.entry(item.room_id) {
            Entry::Occupied(mut o) => {
                let room = o.get_mut();
                let mut msg = MessageInfo::from(item);
                let new_offset = (timestamp - msg.send_at.timestamp_millis()) / 86400000;
                if new_offset != offset {
                    msg.divide = true;
                    offset = new_offset;
                }
                room.messages.push(msg);
                room.members.sort_by(cmp_member);
            }
            Entry::Vacant(_) => {
                return Err(AppError::Database);
            }
        }
    }

    let mut rooms: Vec<RoomInfo> = rooms_map.into_values().collect();
    rooms.sort_by(cmp_room);
    Ok(rooms)
}

// ---------------- UTILS ---------------- //
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

fn rank_number(rank: &String) -> i64 {
    match rank.as_str() {
        "owner" => 1,
        "manager" => 2,
        "member" => 5,
        _ => 10,
    }
}

pub fn cmp_member(a: &MemberInfo, b: &MemberInfo) -> Ordering {
    let a_rank = rank_number(&a.rank);
    let b_rank = rank_number(&b.rank);
    a_rank.cmp(&b_rank)
}
