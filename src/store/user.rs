use super::{Store, User, UserInfo};
use crate::core::constant::{
    CATEGORY_PERSONAL, DEFAULT_AVATAR, PERSONAL_ROOM_COVER, PERSONAL_ROOM_NAME, RANK_MEMBER,
    RANK_OWNER,
};
use crate::{
    api::{ChangePasswordRequest, CreateUserRequest, ListUsersRequest, UpdateUserRequest},
    core::{Error, ResultExt},
    util::password::{hash_password, verify_password},
};
use time::OffsetDateTime;

impl Store {
    pub async fn create_user(&self, arg: &CreateUserRequest) -> Result<UserInfo, Error> {
        let hashed_password = hash_password(&arg.password)?;

        let mut transaction = self.pool.begin().await?;

        // create a new personal room
        let room_id = sqlx::query_scalar!(
            r#"
                INSERT INTO rooms
                    (name, cover, category)
                VALUES
                    ($1, $2, $3)
                RETURNING id
            "#,
            PERSONAL_ROOM_NAME,
            PERSONAL_ROOM_COVER,
            CATEGORY_PERSONAL,
        )
        .fetch_one(&mut *transaction)
        .await?;

        // create a new user
        let user = sqlx::query_as!(
            UserInfo,
            r#"
                INSERT INTO users
                    (username, hashed_password, avatar, nickname, role, room_id)
                VALUES
                    ($1, $2, $3, $4, $5, $6)
                RETURNING
                    id, username, avatar, nickname, role, room_id, deleted, create_at
            "#,
            arg.username,
            hashed_password,
            DEFAULT_AVATAR,
            arg.username,
            arg.role,
            room_id,
        )
        .fetch_one(&mut *transaction)
        .await
        .on_constraint("users_username_key")?;

        // add user to the room
        sqlx::query!(
            r#"
                INSERT INTO members
                    (room_id, member_id, rank)
                VALUES
                    ($1, $2, $3)
            "#,
            room_id,
            user.id,
            RANK_MEMBER,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(user)
    }

    pub async fn get_user(&self, user_id: i64) -> Result<UserInfo, Error> {
        sqlx::query_as!(
            UserInfo,
            r#"
                SELECT
                    id, username, avatar, nickname, role, room_id, deleted, create_at
                FROM users
                WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .not_found()
    }

    pub async fn find_user(&self, username: &String) -> Result<Option<User>, Error> {
        let user = sqlx::query_as!(
            User,
            r#"
                SELECT
                    id, username, hashed_password, avatar, nickname,
                    role, room_id, deleted, create_at
                FROM users
                WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn list_users(&self, arg: &ListUsersRequest) -> Result<(i64, Vec<UserInfo>), Error> {
        let limit = arg.page_size;
        let offset = (arg.page_id - 1) * limit;

        let result = sqlx::query_as!(
            ListUsersRow,
            r#"
                SELECT
                    id, username, avatar, nickname, role, room_id,
                    deleted, create_at, count(*) OVER() AS total
                FROM users
                ORDER BY create_at ASC
                LIMIT $1
                OFFSET $2
            "#,
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = match result.get(0) {
            Some(row) => row.total.unwrap_or(0),
            None => 0,
        };
        let arr: Vec<UserInfo> = result.into_iter().map(|row| row.into()).collect();

        Ok((total, arr))
    }

    pub async fn update_user(&self, arg: &UpdateUserRequest) -> Result<UserInfo, Error> {
        let hashed_password: Option<String> = arg
            .password
            .as_ref()
            .map(|v| hash_password(&v))
            .transpose()?;

        sqlx::query_as!(
            UserInfo,
            r#"
                UPDATE users
                SET
                    username = coalesce($1, username),
                    hashed_password = coalesce($2, hashed_password),
                    nickname = coalesce($3, nickname),
                    avatar = coalesce($4, avatar),
                    role = coalesce($5, role),
                    deleted = coalesce($6, deleted)
                WHERE id = $7
                RETURNING
                    id, username, nickname, avatar, role, room_id, deleted, create_at
            "#,
            arg.username,
            hashed_password,
            arg.nickname,
            arg.avatar,
            arg.role,
            arg.deleted,
            arg.user_id
        )
        .fetch_one(&self.pool)
        .await
        .on_constraint("users_username_key")
    }

    pub async fn change_avatar(&self, user_id: i64, avatar: &String) -> Result<String, Error> {
        let old_avatar = sqlx::query_scalar!(
            r#"
                UPDATE users AS x
                SET avatar = $1
                FROM
                    (SELECT id, avatar FROM users where id = $2 FOR UPDATE) AS y
                WHERE x.id = y.id
                RETURNING y.avatar
            "#,
            avatar,
            user_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(old_avatar)
    }

    pub async fn change_password(
        &self,
        user_id: i64,
        req: &ChangePasswordRequest,
    ) -> Result<(), Error> {
        let hashed_password = sqlx::query_scalar!(
            r#"
                SELECT hashed_password
                FROM users
                WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .not_found()?;

        verify_password(&req.old_password, &hashed_password)?;

        let hashed_password = hash_password(&req.new_password)?;
        sqlx::query!(
            r#"
                UPDATE users
                SET hashed_password = $1
                WHERE id = $2
            "#,
            hashed_password,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_user(&self, user_id: i64) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;

        // get user owned rooms
        let mut rooms = sqlx::query_scalar!(
            r#"
                SELECT room_id
                FROM members
                WHERE
                    member_id = $1
                    AND rank = $2
            "#,
            user_id,
            RANK_OWNER,
        )
        .fetch_all(&mut *transaction)
        .await?;

        // delete friendship
        let mut friend_rooms = sqlx::query_scalar!(
            r#"
                DELETE FROM friends
                WHERE
                    requester_id = $1
                    OR addressee_id = $1
                RETURNING room_id
            "#,
            user_id
        )
        .fetch_all(&mut *transaction)
        .await?;

        rooms.append(&mut friend_rooms);

        // delete messages
        sqlx::query!(
            r#"
                DELETE FROM messages
                WHERE
                    sender_id = $1
                    OR room_id = ANY($2::bigint[])
            "#,
            user_id,
            &rooms,
        )
        .execute(&mut *transaction)
        .await?;

        // delete members
        sqlx::query!(
            r#"
                DELETE FROM members
                WHERE
                    member_id = $1
                    OR room_id = ANY($2::bigint[])
            "#,
            user_id,
            &rooms,
        )
        .execute(&mut *transaction)
        .await?;

        // delete user
        let personal_room = sqlx::query_scalar!(
            r#"
                DELETE FROM users
                WHERE id = $1
                RETURNING room_id
            "#,
            user_id,
        )
        .fetch_one(&mut *transaction)
        .await?;

        rooms.push(personal_room);

        // delete rooms
        sqlx::query!(
            r#"
                DELETE FROM rooms
                WHERE id = ANY($1::bigint[])
            "#,
            &rooms,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }
}

// ========================// Conversions //======================== //

struct ListUsersRow {
    id: i64,
    username: String,
    avatar: String,
    nickname: String,
    role: String,
    room_id: i64,
    deleted: bool,
    create_at: OffsetDateTime,
    total: Option<i64>,
}

impl From<ListUsersRow> for UserInfo {
    fn from(v: ListUsersRow) -> Self {
        Self {
            id: v.id,
            username: v.username,
            avatar: v.avatar,
            nickname: v.nickname,
            role: v.role,
            room_id: v.room_id,
            deleted: v.deleted,
            create_at: v.create_at,
        }
    }
}
