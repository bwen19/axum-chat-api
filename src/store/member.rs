//! Methods of Store for managing room members

use super::{RoomInfo, Store};
use crate::core::{
    constant::{CATEGORY_PRIVATE, RANK_MEMBER},
    Error, ResultExt,
};
use crate::{
    api::{AddMembersRequest, DeleteMembersRequest},
    store::MemberInfo,
};
use std::collections::{hash_map::Entry, HashMap};
use time::OffsetDateTime;

impl Store {
    /// Add members into the room and return these members info
    pub async fn add_members(&self, req: &AddMembersRequest) -> Result<Vec<MemberInfo>, Error> {
        let rooms_id = vec![req.room_id; req.members_id.len()];
        let ranks = vec![RANK_MEMBER.to_string(); req.members_id.len()];

        let members_id = sqlx::query_scalar!(
            r#"
                INSERT INTO members
                    (room_id, member_id, rank)
                SELECT * FROM
                    UNNEST($1::bigint[], $2::bigint[], $3::varchar[])
                ON CONFLICT (room_id, member_id) DO NOTHING
                RETURNING member_id
            "#,
            &rooms_id,
            &req.members_id,
            &ranks,
        )
        .fetch_all(&self.pool)
        .await?;

        let members = sqlx::query_as!(
            MemberInfo,
            r#"
                SELECT
                    id, nickname AS name, avatar, rank, y.join_at
                FROM members y
                JOIN users u ON y.member_id = u.id
                WHERE
                    y.room_id = $1
                    AND y.member_id = ANY($2::bigint[])
            "#,
            &req.room_id,
            &members_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(members)
    }

    /// Delete room members
    ///
    /// The owner can not be deleted, otherwise the room will be deleted
    pub async fn delete_members(&self, req: &DeleteMembersRequest) -> Result<Vec<i64>, Error> {
        let members_id = sqlx::query_scalar!(
            r#"
                DELETE FROM members
                WHERE
                    room_id = $1
                    AND member_id = ANY($2::bigint[])
                    AND rank <> 'owner'
                RETURNING member_id
            "#,
            req.room_id,
            &req.members_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(members_id)
    }

    /// Get the rank of a member in the room
    pub async fn get_rank(&self, member_id: i64, room_id: i64) -> Result<String, Error> {
        sqlx::query_scalar!(
            r#"
                SELECT rank
                FROM members
                WHERE
                    room_id = $1
                    AND member_id = $2
            "#,
            room_id,
            member_id,
        )
        .fetch_one(&self.pool)
        .await
        .not_found()
    }

    pub async fn get_user_rooms_members(
        &self,
        user_id: i64,
    ) -> Result<HashMap<i64, RoomInfo>, Error> {
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

        Ok(rooms)
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
