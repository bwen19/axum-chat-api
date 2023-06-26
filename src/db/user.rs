use super::{
    model::{User, UserInfo},
    Store,
};
use crate::{
    api::{
        auth::RegisterRequest,
        user::{ChangePasswordRequest, CreateUserRequest, ListUsersRequest, UpdateUserRequest},
    },
    error::AppError,
    util::password::verify_password,
};
use crate::{
    error::{AppResult, ResultExt},
    util::password::hash_password,
};
use chrono::{DateTime, Utc};

// ========================// User Store //======================== //

impl Store {
    /// Create a new user
    ///
    /// The username has a unique constraint
    pub async fn create_user(&self, arg: &CreateUserParam) -> AppResult<UserInfo> {
        let hashed_password = hash_password(&arg.password)?;

        let mut transaction = self.pool.begin().await?;

        // create a new personal room
        let room_id = sqlx::query_scalar!(
            r#"
                INSERT INTO rooms (name, cover, category)
                VALUES ('My device', '/cover/personal', 'personal')
                RETURNING id
            "#,
        )
        .fetch_one(&mut *transaction)
        .await
        .exactly_one()?;

        // create new user
        let user = sqlx::query_as!(
            UserInfo,
            r#"
                INSERT INTO users (username, hashed_password, nickname, role, room_id, avatar)
                VALUES ($1, $2, $3, $4, $5, '/avatar/default')
                RETURNING id, username, nickname, avatar, bio, role,
                    deleted, create_at
            "#,
            arg.username,
            hashed_password,
            arg.username,
            arg.role,
            room_id,
        )
        .fetch_one(&mut *transaction)
        .await
        .on_constraint("users_username_key", &arg.username)?;

        // add user to the room
        sqlx::query!(
            r#"
                INSERT INTO room_members (room_id, member_id)
                VALUES ($1, $2)
            "#,
            room_id,
            user.id,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(user)
    }

    /// Delete a list of users by IDs
    pub async fn delete_users(&self, user_ids: &Vec<i64>) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM rooms
                WHERE id IN (
                    SELECT room_id
                    FROM users
                    WHERE id = ANY($1::bigint[])
                )
            "#,
            user_ids
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update the data of a user if provided with a value
    pub async fn update_user(&self, arg: &UpdateUserRequest) -> AppResult<UserInfo> {
        let hashed_password: Option<String> = arg
            .password
            .as_ref()
            .map(|pw| hash_password(&pw))
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
                    bio = coalesce($5, bio),
                    role = coalesce($6, role),
                    deleted = coalesce($7, deleted)
                WHERE id = $8
                RETURNING id, username, nickname, avatar, bio, role,
                    deleted, create_at
            "#,
            arg.username,
            hashed_password,
            arg.nickname,
            arg.avatar,
            arg.bio,
            arg.role,
            arg.deleted,
            arg.user_id
        )
        .fetch_one(&self.pool)
        .await
        .on_constraint(
            "users_username_key",
            &arg.username.as_deref().unwrap_or("name"),
        )
    }

    /// Change the avatar of user
    pub async fn change_avatar(&self, user_id: i64, avatar: &String) -> AppResult<String> {
        let old_avatar = sqlx::query_scalar!(
            r#"
                UPDATE users AS x
                SET avatar = $1
                FROM (SELECT id, avatar FROM users where id = $2 FOR UPDATE) AS y
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

    /// Change the password of user
    pub async fn change_password(
        &self,
        user_id: i64,
        req: &ChangePasswordRequest,
    ) -> AppResult<()> {
        let hashed_password = sqlx::query_scalar!(
            r#"
                SELECT hashed_password
                FROM users WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()?;

        if !verify_password(&req.old_password, &hashed_password)? {
            return Err(AppError::WrongPassword);
        }

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

    /// Get the info of a user by ID
    pub async fn get_user(&self, user_id: i64) -> AppResult<UserInfo> {
        sqlx::query_as!(
            UserInfo,
            r#"
                SELECT id, username, nickname, avatar, bio, role,
                    deleted, create_at
                FROM users
                WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()
    }

    /// Get the entity of a user by username, return None if not exists
    pub async fn find_user(&self, username: &String) -> AppResult<Option<User>> {
        let user = sqlx::query_as!(
            User,
            r#"
                SELECT id, username, hashed_password, nickname, avatar,
                    bio, role, deleted, room_id, create_at
                FROM users
                WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Get a list of user's information
    pub async fn list_users(&self, arg: &ListUsersParam) -> AppResult<(i64, Vec<UserInfo>)> {
        let result = sqlx::query_as!(
            ListUsersRow,
            r#"
                SELECT id, username, nickname, avatar, bio, role,
                    deleted, create_at, count(*) OVER() AS total
                FROM users
                WHERE $1 OR username LIKE $2
                ORDER BY create_at ASC
                LIMIT $3
                OFFSET $4
            "#,
            arg.any_keyword,
            arg.keyword,
            arg.limit,
            arg.offset,
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
}

// ========================// Conversions //======================== //

pub struct CreateUserParam {
    username: String,
    password: String,
    role: String,
}

impl From<RegisterRequest> for CreateUserParam {
    fn from(value: RegisterRequest) -> Self {
        Self {
            username: value.username,
            password: value.password,
            role: "user".to_owned(),
        }
    }
}

impl From<CreateUserRequest> for CreateUserParam {
    fn from(value: CreateUserRequest) -> Self {
        Self {
            username: value.username,
            password: value.password,
            role: value.role,
        }
    }
}

pub struct ListUsersParam {
    limit: i64,
    offset: i64,
    any_keyword: bool,
    keyword: String,
}

impl From<ListUsersRequest> for ListUsersParam {
    fn from(value: ListUsersRequest) -> Self {
        let page_id = value.page_id.unwrap_or(1);
        let page_size = value.page_size.unwrap_or(10);

        Self {
            limit: page_size,
            offset: (page_id - 1) * page_size,
            any_keyword: value.keyword.is_none(),
            keyword: value.keyword.unwrap_or(String::default()),
        }
    }
}

struct ListUsersRow {
    id: i64,
    username: String,
    nickname: String,
    avatar: String,
    bio: String,
    role: String,
    deleted: bool,
    create_at: DateTime<Utc>,
    total: Option<i64>,
}

impl From<ListUsersRow> for UserInfo {
    fn from(row: ListUsersRow) -> Self {
        Self {
            id: row.id,
            username: row.username,
            nickname: row.nickname,
            avatar: row.avatar,
            bio: row.bio,
            role: row.role,
            deleted: row.deleted,
            create_at: row.create_at,
        }
    }
}
