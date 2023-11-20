//! Methods of Store for managing chat rooms

use super::model::{MemberInfo, Room, RoomInfo};
use super::{cmp_member, Store};
use crate::api::{NewRoomRequest, UpdateRoomResponse, UpdateRoomResquest};
use crate::core::constant::{CATEGORY_PUBLIC, PUBLIC_ROOM_COVER, RANK_MEMBER, RANK_OWNER};
use crate::core::{Error, ResultExt};
use redis::AsyncCommands;

impl Store {
    /// Create room and add initial members
    pub async fn create_room(
        &self,
        owner_id: i64,
        req: &NewRoomRequest,
    ) -> Result<RoomInfo, Error> {
        let mut ranks = vec![RANK_OWNER.to_string()];
        let mut members_id = vec![owner_id];
        for id in &req.members_id {
            ranks.push(RANK_MEMBER.to_string());
            members_id.push(*id);
        }

        let mut transaction = self.pool.begin().await?;

        // create new room
        let room = sqlx::query_as!(
            Room,
            r#"
                INSERT INTO rooms
                    (name, cover, category)
                VALUES
                    ($1, $2, $3)
                RETURNING
                    id, name, cover, category, create_at
            "#,
            req.name,
            PUBLIC_ROOM_COVER,
            CATEGORY_PUBLIC,
        )
        .fetch_one(&mut *transaction)
        .await?;

        // make input of room members
        let rooms_id = vec![room.id; members_id.len()];

        // insert members and get members info
        let mut members = sqlx::query_as!(
            MemberInfo,
            r#"
                WITH insert_cte AS (
                    INSERT INTO members
                        (room_id, member_id, rank)
                    SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::varchar[])
                    RETURNING
                        member_id, rank, join_at
                )
                SELECT
                    u.id, u.nickname AS name, u.avatar, y.rank, y.join_at
                FROM insert_cte AS y
                JOIN users AS u ON y.member_id = u.id
            "#,
            &rooms_id,
            &members_id,
            &ranks,
        )
        .fetch_all(&mut *transaction)
        .await?;

        transaction.commit().await?;

        members.sort_by(cmp_member);
        Ok((room, members).into())
    }

    /// Get room info when join a new room
    pub async fn get_room(&self, room_id: i64) -> Result<RoomInfo, Error> {
        let room = sqlx::query_as!(
            Room,
            r#"
                SELECT
                    id, name, cover, category, create_at
                FROM rooms
                WHERE id = $1
            "#,
            room_id,
        )
        .fetch_one(&self.pool)
        .await
        .not_found()?;

        // get members info
        let mut members = sqlx::query_as!(
            MemberInfo,
            r#"
                SELECT
                    u.id, u.nickname AS name, u.avatar, y.rank, y.join_at
                FROM members AS y
                JOIN users AS u ON y.member_id = u.id
                WHERE y.room_id = $1
            "#,
            room_id,
        )
        .fetch_all(&self.pool)
        .await?;

        members.sort_by(cmp_member);
        Ok((room, members).into())
    }

    /// Delete the room and return the member's id
    ///
    /// In general, only the admin and owner of the room can delete the room.
    /// So, one should check that before call the function.
    pub async fn delete_room(&self, room_id: i64) -> Result<Vec<i64>, Error> {
        let mut transaction = self.pool.begin().await?;

        // delete members and return all members ID
        let members_id = sqlx::query_scalar!(
            r#"
                DELETE FROM members
                WHERE room_id = $1
                RETURNING member_id
            "#,
            room_id
        )
        .fetch_all(&mut *transaction)
        .await?;

        // delete room
        sqlx::query!(
            r#"
                DELETE FROM rooms
                WHERE id = $1
            "#,
            room_id,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        // delete messages stored in redis
        let mut con = self.client.get_async_connection().await?;
        let key = format!("room:{}", room_id);
        con.del(key).await?;

        Ok(members_id)
    }

    pub async fn change_cover(&self, room_id: i64, cover: &str) -> Result<String, Error> {
        let old_cover = sqlx::query_scalar!(
            r#"
                UPDATE rooms AS x
                SET cover = $1
                FROM
                    (SELECT id, cover FROM rooms where id = $2 FOR UPDATE) AS y
                WHERE x.id = y.id
                RETURNING y.cover
            "#,
            cover,
            room_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(old_cover)
    }

    pub async fn update_room(&self, req: &UpdateRoomResquest) -> Result<UpdateRoomResponse, Error> {
        sqlx::query_as!(
            UpdateRoomResponse,
            r#"
                UPDATE rooms
                SET name = $1
                WHERE id = $2
                RETURNING
                    id AS room_id, name
            "#,
            req.name,
            req.room_id,
        )
        .fetch_one(&self.pool)
        .await
        .not_found()
    }
}
