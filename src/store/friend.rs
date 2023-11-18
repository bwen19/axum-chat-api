//! Methods of Store for managing friends

use super::{
    model::{Friend, FriendInfo, MemberInfo, RoomInfo},
    Store,
};
use crate::core::constant::{
    CATEGORY_PRIVATE, PRIVATE_ROOM_COVER, PRIVATE_ROOM_NAME, RANK_MEMBER, STATUS_ACCEPTED,
    STATUS_ADDING, STATUS_DELETED,
};
use crate::core::{Error, ResultExt};
use std::collections::HashMap;
use time::OffsetDateTime;

impl Store {
    /// Get the friendship between the two users
    pub async fn get_friend(&self, user0_id: i64, user1_id: i64) -> Result<Option<Friend>, Error> {
        let friend = sqlx::query_as!(
            Friend,
            r#"
                SELECT * FROM friends
                WHERE
                    (requester_id = $1 AND addressee_id = $2)
                    OR (requester_id = $2 AND addressee_id = $1)
            "#,
            user0_id,
            user1_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(friend)
    }

    /// Get the friendship detail between requester and addressee
    pub async fn get_friend_info(
        &self,
        friend: &Friend,
    ) -> Result<(FriendInfo, FriendInfo), Error> {
        let friends = sqlx::query_as!(
            BiFriendInfoRow,
            r#"
                SELECT
                    f.room_id, f.status, f.create_at,
                    r.id AS r_id, r.username AS r_username,
                    r.nickname AS r_nickname, r.avatar AS r_avatar,
                    a.id AS a_id, a.username AS a_username,
                    a.nickname AS a_nickname, a.avatar AS a_avatar
                FROM friends AS f
                    JOIN users AS r ON r.id = f.requester_id
                    JOIN users AS a ON a.id = f.addressee_id
                WHERE
                    requester_id = $1 AND addressee_id = $2
                LIMIT 1;
            "#,
            friend.requester_id,
            friend.addressee_id,
        )
        .fetch_one(&self.pool)
        .await
        .not_found()?;

        Ok(friends.into())
    }

    /// Get the friendship detail between requester and addressee
    pub async fn get_friend_room(&self, friend: &Friend) -> Result<(RoomInfo, RoomInfo), Error> {
        let room_members = sqlx::query_as!(
            RoomMemberRow,
            r#"
                SELECT
                    m.room_id, r.category, r.create_at, m.member_id,
                    u.nickname, u.avatar, m.rank, m.join_at
                FROM
                    members AS m
                    JOIN rooms AS r ON r.id = m.room_id
                    JOIN users AS u ON u.id = m.member_id
                WHERE m.room_id = $1;
            "#,
            friend.room_id,
        )
        .fetch_all(&self.pool)
        .await?;

        if room_members.len() != 2 {
            return Err(Error::Database);
        }

        let mut rooms: HashMap<i64, RoomInfo> = HashMap::new();
        let mut members = Vec::new();

        for v in room_members {
            let member = MemberInfo {
                id: v.member_id,
                name: v.nickname.clone(),
                avatar: v.avatar.clone(),
                rank: v.rank,
                join_at: v.join_at,
            };
            members.push(member);

            let room = RoomInfo {
                id: v.room_id,
                name: v.nickname,
                cover: v.avatar,
                category: v.category,
                create_at: v.create_at,
                unreads: 0,
                members: Vec::new(),
                messages: Vec::new(),
            };
            rooms.insert(v.member_id, room);
        }

        let mut room0 = rooms.remove(&friend.requester_id).ok_or(Error::Database)?;
        room0.members = members.clone();

        let mut room1 = rooms.remove(&friend.addressee_id).ok_or(Error::Database)?;
        room1.members = members;

        Ok((room0, room1))
    }

    /// Create a new friendship which does not exist in the database
    pub async fn create_friend(
        &self,
        requester_id: i64,
        addressee_id: i64,
    ) -> Result<Friend, Error> {
        let mut transaction = self.pool.begin().await?;

        // create a new private room
        let room_id = sqlx::query_scalar!(
            r#"
                INSERT INTO rooms
                    (name, cover, category)
                VALUES
                    ($1, $2, $3)
                RETURNING
                    id
            "#,
            PRIVATE_ROOM_NAME,
            PRIVATE_ROOM_COVER,
            CATEGORY_PRIVATE,
        )
        .fetch_one(&mut *transaction)
        .await
        .not_found()?;

        // create new friendship
        let friend = sqlx::query_as!(
            Friend,
            r#"
                INSERT INTO friends
                    (requester_id, addressee_id, room_id, status)
                VALUES
                    ($1, $2, $3, $4)
                RETURNING *
            "#,
            requester_id,
            addressee_id,
            room_id,
            STATUS_ADDING,
        )
        .fetch_one(&mut *transaction)
        .await
        .not_found()?;

        transaction.commit().await?;

        Ok(friend)
    }

    /// Update a deleted friendship
    pub async fn update_friend(
        &self,
        requester_id: i64,
        addressee_id: i64,
    ) -> Result<Friend, Error> {
        let row = sqlx::query_as!(
            Friend,
            r#"
                UPDATE friends
                SET
                    requester_id = $1,
                    addressee_id = $2,
                    status = $3
                WHERE
                    (requester_id = $1 AND addressee_id = $2)
                    OR (requester_id = $2 AND addressee_id = $1)
                RETURNING *

            "#,
            requester_id,
            addressee_id,
            STATUS_ADDING,
        )
        .fetch_one(&self.pool)
        .await
        .not_found()?;

        Ok(row)
    }

    /// Accept a friendship
    pub async fn accept_friend(&self, friend: &Friend) -> Result<(), Error> {
        if friend.status != STATUS_ADDING {
            return Err(Error::FriendStatus);
        }

        let mut transaction = self.pool.begin().await?;

        let res = sqlx::query!(
            r#"
                UPDATE friends
                SET status = $1
                WHERE
                    requester_id = $2
                    AND addressee_id = $3
            "#,
            STATUS_ACCEPTED,
            friend.requester_id,
            friend.addressee_id,
        )
        .execute(&mut *transaction)
        .await?;

        if res.rows_affected() != 1 {
            return Err(Error::Database);
        }

        let res = sqlx::query!(
            r#"
                INSERT INTO members
                    (room_id, member_id, rank)
                VALUES
                    ($1, $2, $3), ($4, $5, $6)
            "#,
            friend.room_id,
            friend.requester_id,
            RANK_MEMBER,
            friend.room_id,
            friend.addressee_id,
            RANK_MEMBER,
        )
        .execute(&mut *transaction)
        .await?;

        if res.rows_affected() != 2 {
            return Err(Error::Database);
        }

        transaction.commit().await?;

        Ok(())
    }

    /// Refuse a friendship
    pub async fn refuse_friend(&self, friend: &Friend) -> Result<(), Error> {
        if friend.status != STATUS_ADDING {
            return Err(Error::FriendStatus);
        }

        let res = sqlx::query!(
            r#"
                UPDATE friends
                SET status = $1
                WHERE
                    requester_id = $2
                    AND addressee_id = $3
            "#,
            STATUS_DELETED,
            friend.requester_id,
            friend.addressee_id,
        )
        .execute(&self.pool)
        .await?;

        if res.rows_affected() != 1 {
            return Err(Error::Database);
        }

        Ok(())
    }

    /// Delete a friendship
    pub async fn delete_friend(&self, friend: &Friend) -> Result<(), Error> {
        if friend.status != STATUS_ACCEPTED {
            return Err(Error::FriendStatus);
        }

        let mut transaction = self.pool.begin().await?;

        let res = sqlx::query!(
            r#"
                UPDATE friends
                SET status = $1
                WHERE
                    requester_id = $2
                    AND addressee_id = $3
            "#,
            STATUS_DELETED,
            friend.requester_id,
            friend.addressee_id,
        )
        .execute(&mut *transaction)
        .await
        .not_found()?;

        if res.rows_affected() != 1 {
            return Err(Error::Database);
        }

        let res = sqlx::query!(
            r#"
                DELETE FROM members
                WHERE room_id = $1
            "#,
            friend.room_id,
        )
        .execute(&mut *transaction)
        .await?;

        if res.rows_affected() != 2 {
            return Err(Error::Database);
        }

        transaction.commit().await?;

        Ok(())
    }
}

// ========================// Conversions //======================== //

struct BiFriendInfoRow {
    room_id: i64,
    status: String,
    create_at: OffsetDateTime,
    r_id: i64,
    r_username: String,
    r_nickname: String,
    r_avatar: String,
    a_id: i64,
    a_username: String,
    a_nickname: String,
    a_avatar: String,
}

impl From<BiFriendInfoRow> for (FriendInfo, FriendInfo) {
    fn from(row: BiFriendInfoRow) -> Self {
        let requester = FriendInfo {
            id: row.r_id,
            username: row.r_username,
            nickname: row.r_nickname,
            avatar: row.r_avatar,
            status: row.status.clone(),
            room_id: row.room_id,
            first: true,
            create_at: row.create_at.clone(),
        };
        let addressee = FriendInfo {
            id: row.a_id,
            username: row.a_username,
            nickname: row.a_nickname,
            avatar: row.a_avatar,
            status: row.status,
            room_id: row.room_id,
            first: false,
            create_at: row.create_at,
        };
        (requester, addressee)
    }
}

struct RoomMemberRow {
    room_id: i64,
    category: String,
    create_at: OffsetDateTime,
    member_id: i64,
    nickname: String,
    avatar: String,
    rank: String,
    join_at: OffsetDateTime,
}
