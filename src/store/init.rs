//! Methods of Store for initialization

use super::{cmp_member, cmp_room, FriendInfo, MessageInfo, RoomInfo, Store};
use crate::api::CreateUserRequest;
use crate::core::constant::ROLE_ADMIN;
use crate::core::{
    constant::{STATUS_ACCEPTED, STATUS_ADDING},
    Error,
};
use redis::AsyncCommands;
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
                role: ROLE_ADMIN.to_owned(),
            };

            self.create_user(&arg)
                .await
                .expect("failed to create admin account");
        }

        tracing::info!("db was successfully initialized");
    }

    /// Get all the user joined rooms
    pub async fn get_user_rooms(
        &self,
        user_id: i64,
        timestamp: i64,
    ) -> Result<Vec<RoomInfo>, Error> {
        // get user's rooms and members of each room
        let rooms = self.get_user_rooms_members(user_id).await?;

        let mut rooms_info = Vec::new();
        let mut con = self.client.get_async_connection().await?;

        for (i, mut room) in rooms {
            let key = format!("room:{}", i);
            let messages: Vec<String> = con.lrange(key, 0, 19).await?;
            let mut offset = 0_i64;
            for m in messages.iter().rev() {
                let mut msg = serde_json::from_str::<MessageInfo>(m)?;
                let new_offset = (timestamp - msg.send_at.unix_timestamp() * 1000) / 86400000;
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

    /// Get all friends of the user
    pub async fn get_user_friends(&self, user_id: i64) -> Result<Vec<FriendInfo>, Error> {
        let mut friends: Vec<FriendInfo> = sqlx::query_as!(
            FriendInfoRow,
            r#"
                SELECT
                    f.room_id, f.status, f.create_at, u.id, u.username,
                    u.nickname, u.avatar, (f.addressee_id = $1) AS first
                FROM friends AS f
                    JOIN users AS u ON u.id = f.addressee_id
                WHERE
                    f.requester_id = $1
                    AND status IN ($2, $3)
            "#,
            user_id,
            STATUS_ADDING,
            STATUS_ACCEPTED,
        )
        .fetch_all(&self.pool)
        .await
        .map(|arr| arr.into_iter().map(|x| x.into()).collect())?;

        let mut other_friends: Vec<FriendInfo> = sqlx::query_as!(
            FriendInfoRow,
            r#"
                SELECT
                    f.room_id, f.status, f.create_at, u.id, u.username,
                    u.nickname, u.avatar, (f.addressee_id = $1) AS first
                FROM friends AS f
                    JOIN users AS u ON u.id = f.requester_id
                WHERE
                    f.addressee_id = $1
                    AND status IN ($2, $3)
            "#,
            user_id,
            STATUS_ADDING,
            STATUS_ACCEPTED,
        )
        .fetch_all(&self.pool)
        .await
        .map(|arr| arr.into_iter().map(|x| x.into()).collect())?;

        friends.append(&mut other_friends);
        Ok(friends)
    }
}

struct FriendInfoRow {
    id: i64,
    username: String,
    nickname: String,
    avatar: String,
    status: String,
    room_id: i64,
    first: Option<bool>,
    create_at: OffsetDateTime,
}

impl From<FriendInfoRow> for FriendInfo {
    fn from(v: FriendInfoRow) -> Self {
        Self {
            id: v.id,
            username: v.username,
            nickname: v.nickname,
            avatar: v.avatar,
            status: v.status,
            room_id: v.room_id,
            first: v.first.unwrap_or(false),
            create_at: v.create_at,
        }
    }
}
