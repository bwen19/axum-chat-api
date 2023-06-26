use super::validator as VAL;
use crate::{
    db::model::UserInfo,
    error::{AppError, AppResult},
    extractor::{AdminGuard, AuthGuard, ValidatedJson, ValidatedQuery},
    util, AppState,
};
use axum::{
    extract::{Multipart, State},
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use validator::Validate;

// ========================// User Router //======================== //

/// Create user router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/user",
            get(list_users)
                .post(create_user)
                .patch(update_user)
                .delete(delete_users),
        )
        .route("/user/username", get(find_user))
        .route("/user/password", patch(change_password))
        .route("/user/avatar", post(change_avatar))
}

// ========================// Create User //======================== //

#[derive(Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub username: String,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub password: String,
    #[validate(custom = "VAL::validate_user_role")]
    pub role: String,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub user: UserInfo,
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
    ValidatedJson(req): ValidatedJson<CreateUserRequest>,
) -> AppResult<Json<CreateUserResponse>> {
    let user = state.db.create_user(&req.into()).await?;

    Ok(Json(CreateUserResponse { user }))
}

// ========================// Delete Users //======================== //

#[derive(Deserialize, Validate)]
pub struct DeleteUsersRequest {
    #[validate(custom = "VAL::validate_id_vec")]
    pub user_ids: Vec<i64>,
}

async fn delete_users(
    State(state): State<Arc<AppState>>,
    AdminGuard(claims): AdminGuard,
    ValidatedJson(req): ValidatedJson<DeleteUsersRequest>,
) -> AppResult<()> {
    if req.user_ids.contains(&claims.user_id) {
        return Err(AppError::DeleteUserSelf);
    }
    state.db.delete_users(&req.user_ids).await?;

    Ok(())
}

// ========================// Update User //======================== //

#[derive(Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(range(min = 1, message = "user id is invalid"))]
    pub user_id: i64,
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub username: Option<String>,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub password: Option<String>,
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub nickname: Option<String>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub avatar: Option<String>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub bio: Option<String>,
    #[validate(custom = "VAL::validate_user_role")]
    pub role: Option<String>,
    pub deleted: Option<bool>,
}

#[derive(Serialize)]
pub struct UpdateUserResponse {
    pub user: UserInfo,
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    AuthGuard(claims): AuthGuard,
    ValidatedJson(req): ValidatedJson<UpdateUserRequest>,
) -> AppResult<Json<UpdateUserResponse>> {
    if !claims.is_admin()
        && (req.role.is_some()
            || req.deleted.is_some()
            || req.password.is_some()
            || req.user_id != claims.user_id)
    {
        return Err(AppError::Forbidden);
    }

    let user = state.db.update_user(&req).await?;

    Ok(Json(UpdateUserResponse { user }))
}

// ========================// Change Avatar //======================== //

#[derive(Serialize)]
pub struct ChangeAvatarResponse {
    pub avatar: String,
}

async fn change_avatar(
    State(state): State<Arc<AppState>>,
    AuthGuard(claims): AuthGuard,
    mut multipart: Multipart,
) -> AppResult<Json<ChangeAvatarResponse>> {
    let avatar = util::common::generate_avatar_name(claims.user_id);

    if let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(content_type) = field.content_type() {
            if !content_type.starts_with("image") {
                return Err(AppError::InvalidFile);
            }
        } else {
            return Err(AppError::InvalidFile);
        }

        let path = Path::new(&state.config.public_directory).join(&avatar[1..]);
        let data = field.bytes().await.unwrap();
        tokio::fs::write(&path, &data).await?;

        let old_avatar = state.db.change_avatar(claims.user_id, &avatar).await?;
        if !old_avatar.ends_with("default") {
            let path = Path::new(&state.config.public_directory).join(&old_avatar[1..]);
            if path.is_file() {
                tokio::fs::remove_file(&path).await?;
            }
        }
    } else {
        return Err(AppError::InvalidFile);
    }

    Ok(Json(ChangeAvatarResponse { avatar }))
}

// ========================// Change Password //======================== //

#[derive(Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub old_password: String,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub new_password: String,
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    AuthGuard(claims): AuthGuard,
    ValidatedJson(req): ValidatedJson<ChangePasswordRequest>,
) -> AppResult<()> {
    state.db.change_password(claims.user_id, &req).await?;

    Ok(())
}

// ========================// List User //======================== //

#[derive(Deserialize, Validate)]
pub struct ListUsersRequest {
    #[validate(range(min = 1, message = "must be greater than 1"))]
    pub page_id: Option<i64>,
    #[validate(range(min = 5, max = 50, message = "must be between 5 and 50"))]
    pub page_size: Option<i64>,
    #[validate(length(min = 1, max = 50, message = "must be between 1 and 50 characters"))]
    pub keyword: Option<String>,
}

#[derive(Serialize)]
pub struct ListUsersResponse {
    pub total: i64,
    pub users: Vec<UserInfo>,
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
    ValidatedQuery(req): ValidatedQuery<ListUsersRequest>,
) -> AppResult<Json<ListUsersResponse>> {
    let (total, users) = state.db.list_users(&req.into()).await?;

    Ok(Json(ListUsersResponse { total, users }))
}

// ========================// Get User by name //======================== //

#[derive(Deserialize, Validate)]
pub struct GetUserByNameRequest {
    #[validate(length(min = 1, max = 50, message = "must be between 1 and 50 characters"))]
    pub username: String,
}

#[derive(Serialize)]
pub struct GetUserByNameResponse {
    pub user: Option<UserInfo>,
}

async fn find_user(
    State(state): State<Arc<AppState>>,
    AuthGuard(_): AuthGuard,
    ValidatedQuery(req): ValidatedQuery<GetUserByNameRequest>,
) -> AppResult<Json<GetUserByNameResponse>> {
    let user = state.db.find_user(&req.username).await?.map(|u| u.into());

    Ok(Json(GetUserByNameResponse { user }))
}
