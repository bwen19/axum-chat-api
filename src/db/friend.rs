use super::{
    model::{FriendInfo, FriendShip, MemberInfo, RoomInfo},
    Store,
};
use crate::error::{AppError, AppResult, ResultExt};
use chrono::{DateTime, Utc};

// ========================// Friend Store //======================== //

impl Store {
    /// Get the relationship between two the users
    pub async fn get_friend(
        &self,
        user_id: i64,
        friend_id: i64,
    ) -> AppResult<Option<FriendShip>> {
        let result = sqlx::query_as!(
            FriendShip,
            r#"
                SELECT user_id, friend_id, room_id, status, create_at
                FROM friendships
                WHERE (user_id = $1 AND friend_id = $2)
                    OR (user_id = $2 AND friend_id = $1)
            "#,
            user_id,
            friend_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Create a new friendship which does not exist in the database
    ///
    /// One should check the friendship before create it
    pub async fn create_friend(&self, user_id: i64, friend_id: i64) -> AppResult<BiFriendInfo> {
        let mut transaction = self.pool.begin().await?;

        // create a new private room
        let room_id = sqlx::query_scalar!(
            r#"
                INSERT INTO rooms (name, category)
                VALUES ('Friend', 'private')
                RETURNING id
            "#,
        )
        .fetch_one(&mut *transaction)
        .await
        .exactly_one()?;

        // create new friendship
        let row = sqlx::query_as!(
            BiFriendInfoRow,
            r#"
                WITH insert_cte AS (
                    INSERT INTO friendships (user_id, friend_id, room_id)
                    VALUES ($1, $2, $3)
                    RETURNING user_id, friend_id, room_id, status, create_at
                )
                SELECT ic.room_id, status, ic.create_at, user_id, u1.username AS u_username,
                    u1.nickname AS u_nickname, u1.avatar AS u_avatar, u1.bio AS u_bio,
                    friend_id, u2.username AS f_username, u2.nickname AS f_nickname,
                    u2.avatar AS f_avatar, u2.bio AS f_bio
                FROM insert_cte AS ic
                INNER JOIN users AS u1 ON u1.id = ic.user_id
                INNER JOIN users AS u2 ON u2.id = ic.friend_id
            "#,
            user_id,
            friend_id,
            room_id
        )
        .fetch_one(&mut *transaction)
        .await
        .exactly_one()?;

        transaction.commit().await?;

        Ok(row.into())
    }

    /// Change the status of deleted to adding
    ///
    /// Used when adding an existed friendship
    pub async fn adding_friend(&self, user_id: i64, friend_id: i64) -> AppResult<BiFriendInfo> {
        let row = sqlx::query_as!(
            BiFriendInfoRow,
            r#"
                WITH update_cte AS (
                    UPDATE friendships
                    SET user_id = $1,
                        friend_id = $2,
                        status = 'adding'
                    WHERE (user_id = $1 AND friend_id = $2 AND status = 'deleted')
                        OR (user_id = $2 AND friend_id = $1 AND status = 'deleted')
                    RETURNING user_id, friend_id, room_id, status, create_at
                )
                SELECT uc.room_id, status, uc.create_at, user_id, u1.username AS u_username,
                    u1.nickname AS u_nickname, u1.avatar AS u_avatar, u1.bio AS u_bio,
                    friend_id, u2.username AS f_username, u2.nickname AS f_nickname,
                    u2.avatar AS f_avatar, u2.bio AS f_bio
                FROM update_cte AS uc
                INNER JOIN users AS u1 ON u1.id = uc.user_id
                INNER JOIN users AS u2 ON u2.id = uc.friend_id
            "#,
            user_id,
            friend_id,
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()?;

        Ok(row.into())
    }

    /// Accept a friendship
    pub async fn accept_friend(
        &self,
        user_id: i64,
        friend_id: i64,
    ) -> AppResult<(BiFriendInfo, BiRoomInfo)> {
        let mut transaction = self.pool.begin().await?;

        let row = sqlx::query_as!(
            BiFriendInfoRow,
            r#"
                WITH update_cte AS (
                    UPDATE friendships
                    SET status = 'accepted'
                    WHERE user_id = $1 AND friend_id = $2 AND status = 'adding'
                    RETURNING user_id, friend_id, room_id, status, create_at
                )
                SELECT uc.room_id, status, uc.create_at, user_id, u1.username AS u_username,
                    u1.nickname AS u_nickname, u1.avatar AS u_avatar, u1.bio AS u_bio,
                    friend_id, u2.username AS f_username, u2.nickname AS f_nickname,
                    u2.avatar AS f_avatar, u2.bio AS f_bio
                FROM update_cte AS uc
                INNER JOIN users AS u1 ON u1.id = uc.user_id
                INNER JOIN users AS u2 ON u2.id = uc.friend_id
            "#,
            user_id,
            friend_id,
        )
        .fetch_one(&mut *transaction)
        .await
        .exactly_one()?;

        let rows = sqlx::query_as!(
            RoomMemberRow,
            r#"
                WITH insert_cte AS (
                    INSERT INTO room_members (room_id, member_id)
                    VALUES ($1, $2), ($3, $4)
                    ON CONFLICT (room_id, member_id) DO NOTHING
                    RETURNING room_id, member_id, rank, join_at
                )
                SELECT ic.room_id, ic.rank, ic.join_at, r.category,
                    r.create_at, ic.member_id, u.nickname, u.avatar
                FROM insert_cte AS ic
                INNER JOIN rooms AS r ON r.id = ic.room_id
                INNER JOIN users AS u ON u.id = ic.member_id
            "#,
            row.room_id,
            row.user_id,
            row.room_id,
            row.friend_id,
        )
        .fetch_all(&mut *transaction)
        .await?;

        transaction.commit().await?;
        let friend = BiFriendInfo::from(row);
        let room = convert_room(rows, user_id)?;

        Ok((friend, room))
    }

    /// Refuse a friendship
    pub async fn refuse_friend(&self, user_id: i64, friend_id: i64) -> AppResult<()> {
        sqlx::query_scalar!(
            r#"
                UPDATE friendships
                SET status = 'deleted'
                WHERE user_id = $1 AND friend_id = $2 AND status = 'adding'
                RETURNING user_id
            "#,
            user_id,
            friend_id,
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()?;

        Ok(())
    }

    /// Delete a friendship
    pub async fn delete_friend(&self, user_id: i64, friend_id: i64) -> AppResult<i64> {
        let mut transaction = self.pool.begin().await?;

        let room_id = sqlx::query_scalar!(
            r#"
                UPDATE friendships
                SET status = 'deleted'
                WHERE (user_id = $1 AND friend_id = $2 AND status = 'accepted')
                    OR (user_id = $2 AND friend_id = $1 AND status = 'accepted')
                RETURNING room_id
        "#,
            user_id,
            friend_id,
        )
        .fetch_one(&mut *transaction)
        .await
        .exactly_one()?;

        sqlx::query!(
            r#"
                DELETE FROM room_members
                WHERE room_id = $1 AND member_id = ANY($2::bigint[])
            "#,
            room_id,
            &vec![user_id, friend_id],
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(room_id)
    }

    /// Get all friends of the user
    pub async fn get_user_friends(&self, user_id: i64) -> AppResult<Vec<FriendInfo>> {
        let mut friends: Vec<FriendInfo> = sqlx::query_as!(
            FriendInfoRow,
            r#"
                SELECT f.room_id, f.status, f.create_at, u.id, u.username, u.nickname,
                    u.avatar, u.bio, (f.user_id = $1) AS first
                FROM friendships AS f
                INNER JOIN users AS u ON u.id = f.friend_id
                WHERE user_id = $1 AND status IN ('adding', 'accepted')
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await
        .map(|arr| arr.into_iter().map(|x| x.into()).collect())?;

        let mut friends_other: Vec<FriendInfo> = sqlx::query_as!(
            FriendInfoRow,
            r#"
                SELECT f.room_id, f.status, f.create_at, u.id, u.username, u.nickname,
                    u.avatar, u.bio, (f.user_id = $1) AS first
                FROM friendships AS f
                INNER JOIN users AS u ON u.id = f.user_id
                WHERE f.friend_id = $1 AND status IN ('adding', 'accepted')
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await
        .map(|arr| arr.into_iter().map(|x| x.into()).collect())?;

        friends.append(&mut friends_other);
        Ok(friends)
    }
}

// ========================// Conversions //======================== //

struct BiFriendInfoRow {
    room_id: i64,
    status: String,
    create_at: DateTime<Utc>,
    user_id: i64,
    u_username: String,
    u_nickname: String,
    u_avatar: String,
    u_bio: String,
    friend_id: i64,
    f_username: String,
    f_nickname: String,
    f_avatar: String,
    f_bio: String,
}

pub struct BiFriendInfo {
    pub ufriend: FriendInfo,
    pub ffriend: FriendInfo,
}

impl From<BiFriendInfoRow> for BiFriendInfo {
    fn from(row: BiFriendInfoRow) -> Self {
        Self {
            ffriend: FriendInfo {
                id: row.user_id,
                username: row.u_username,
                nickname: row.u_nickname,
                avatar: row.u_avatar,
                bio: row.u_bio,
                status: row.status.clone(),
                room_id: row.room_id,
                first: false,
                create_at: row.create_at.clone(),
            },
            ufriend: FriendInfo {
                id: row.friend_id,
                username: row.f_username,
                nickname: row.f_nickname,
                avatar: row.f_avatar,
                bio: row.f_bio,
                status: row.status,
                room_id: row.room_id,
                first: true,
                create_at: row.create_at,
            },
        }
    }
}

struct RoomMemberRow {
    room_id: i64,
    category: String,
    create_at: DateTime<Utc>,
    member_id: i64,
    nickname: String,
    avatar: String,
    rank: String,
    join_at: DateTime<Utc>,
}

pub struct BiRoomInfo {
    pub uroom: RoomInfo,
    pub froom: RoomInfo,
}

fn convert_room(room_members: Vec<RoomMemberRow>, user_id: i64) -> AppResult<BiRoomInfo> {
    let mut uroom: Option<RoomInfo> = None;
    let mut froom: Option<RoomInfo> = None;
    let mut members = Vec::new();

    for item in room_members {
        let member = MemberInfo {
            id: item.member_id,
            name: item.nickname.clone(),
            avatar: item.avatar.clone(),
            rank: item.rank,
            join_at: item.join_at,
        };
        members.push(member);

        let room = RoomInfo {
            id: item.room_id,
            name: item.nickname,
            cover: item.avatar,
            category: item.category,
            create_at: item.create_at,
            unreads: 0,
            members: Vec::new(),
            messages: Vec::new(),
        };
        if item.member_id == user_id {
            froom = Some(room);
        } else {
            uroom = Some(room);
        }
    }

    let mut uroom = uroom.ok_or(AppError::Database)?;
    uroom.members = members.clone();
    let mut froom = froom.ok_or(AppError::Database)?;
    froom.members = members;

    Ok(BiRoomInfo { uroom, froom })
}

struct FriendInfoRow {
    id: i64,
    username: String,
    nickname: String,
    avatar: String,
    bio: String,
    status: String,
    room_id: i64,
    first: Option<bool>,
    create_at: DateTime<Utc>,
}

impl From<FriendInfoRow> for FriendInfo {
    fn from(v: FriendInfoRow) -> Self {
        Self {
            id: v.id,
            username: v.username,
            nickname: v.nickname,
            avatar: v.avatar,
            bio: v.bio,
            status: v.status,
            room_id: v.room_id,
            first: v.first.unwrap_or(false),
            create_at: v.create_at,
        }
    }
}
