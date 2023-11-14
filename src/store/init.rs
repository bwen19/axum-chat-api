use super::{cmp_member, cmp_room, MemberInfo, MessageInfo, RoomInfo, Store};
use crate::{
    api::CreateUserRequest,
    core::{constant::CATEGORY_PRIVATE, Error},
};
use redis::AsyncCommands;
use std::collections::{hash_map::Entry, HashMap};
use time::OffsetDateTime;

impl Store {
    /// Initialize database by :
    ///
    /// 1. run db migration up.
    /// 2. create an admin account if not exist.
    pub async fn init(&self) {
        sqlx::migrate!()
            .run(&self.pool)
            .await
            .expect("failed to run migrate up");

        if let Ok(None) = self.find_user("admin").await {
            let arg = CreateUserRequest {
                username: "admin".to_owned(),
                password: "123456".to_owned(),
                role: "admin".to_owned(),
            };

            if let Err(e) = self.create_user(&arg).await {
                match e {
                    Error::UniqueConstraint(_) => {}
                    _ => panic!("failed to create admin account"),
                }
            };
        }

        tracing::info!("db was successfully initialized");
    }

    /// Get a list of rooms that the user has joined
    pub async fn get_user_rooms(
        &self,
        user_id: i64,
        timestamp: i64,
    ) -> Result<Vec<RoomInfo>, Error> {
        // get user's rooms and members of each room
        let members = sqlx::query_as!(
            RoomMemberRow,
            r#"
                WITH rooms_cte AS (
                    SELECT
                        id AS room_id, name, cover, category, create_at
                    FROM rooms
                    WHERE id IN (
                        SELECT room_id
                        FROM members
                        WHERE member_id = $1
                    )
                )
                SELECT
                    room_id, name, cover, category, create_at,
                    member_id, rank, join_at, nickname, avatar
                FROM rooms_cte AS r,
                    LATERAL (
                        SELECT
                            member_id, rank, join_at, nickname, avatar
                        FROM members AS y
                        JOIN users AS u ON y.member_id = u.id
                        WHERE y.room_id = r.room_id
                    ) AS m
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rooms: HashMap<i64, RoomInfo> = HashMap::new();

        for r in members {
            let member = MemberInfo {
                id: r.member_id,
                name: r.nickname,
                avatar: r.avatar,
                rank: r.rank,
                join_at: r.join_at,
            };

            let is_private = r.category == CATEGORY_PRIVATE && member.id != user_id;

            match rooms.entry(r.room_id) {
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
                            id: r.room_id,
                            name: member.name.clone(),
                            cover: member.avatar.clone(),
                            category: r.category,
                            create_at: r.create_at,
                            unreads: 0,
                            members: vec![member],
                            messages: Vec::new(),
                        }
                    } else {
                        RoomInfo {
                            id: r.room_id,
                            name: r.name,
                            cover: r.cover,
                            category: r.category,
                            create_at: r.create_at,
                            unreads: 0,
                            members: vec![member],
                            messages: Vec::new(),
                        }
                    };
                    v.insert(room);
                }
            }
        }

        let mut rooms_info = Vec::new();
        let mut con = self.client.get_async_connection().await?;

        for (i, mut room) in rooms {
            let key = format!("room:{}", i);
            let messages: Vec<String> = con.lrange(key, 0, 19).await?;
            let mut offset = 0_i64;
            for m in messages.iter().rev() {
                let mut msg = serde_json::from_str::<MessageInfo>(m)?;
                let new_offset = (timestamp - msg.send_at.unix_timestamp()) / 86400;
                if new_offset != offset {
                    msg.divide = true;
                    offset = new_offset;
                }
                room.messages.push(msg);
                room.members.sort_by(cmp_member);
            }
            rooms_info.push(room);
        }

        rooms_info.sort_by(cmp_room);
        Ok(rooms_info)
    }
}

struct RoomMemberRow {
    room_id: i64,
    name: String,
    cover: String,
    category: String,
    create_at: OffsetDateTime,
    member_id: i64,
    nickname: String,
    avatar: String,
    rank: String,
    join_at: OffsetDateTime,
}
