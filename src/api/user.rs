//! Handlers for user accounts

use super::{
    extractor::{AdminGuard, AuthGuard, CookieGuard, ValidJson, ValidQuery},
    AppState, ChangeAvatarResponse, ChangePasswordRequest, CreateUserRequest, CreateUserResponse,
    FindUserResponse, ListUsersRequest, ListUsersResponse, UpdateUserRequest, UpdateUserResponse,
};
use crate::{
    core::constant::{DEFAULT_AVATAR, IMAGE_KEY, ROLE_ADMIN},
    core::Error,
    util,
};
use axum::{
    extract::{Multipart, Path, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use std::{path, sync::Arc};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/user",
            get(list_users).post(create_user).patch(update_user),
        )
        .route("/user/:id", delete(delete_user))
        .route("/user/name/:username", get(find_user))
        .route("/user/password", patch(change_password))
        .route("/user/avatar", post(change_avatar))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
    ValidJson(req): ValidJson<CreateUserRequest>,
) -> Result<Json<CreateUserResponse>, Error> {
    let user = state.db.create_user(&req).await?;
    let rsp = CreateUserResponse { user };
    Ok(Json(rsp))
}

async fn delete_user(
    State(state): State<Arc<AppState>>,
    AdminGuard(claims): AdminGuard,
    Path(user_id): Path<i64>,
) -> Result<(), Error> {
    if user_id == claims.user_id || user_id < 0 {
        return Err(Error::Forbidden);
    }
    state.db.delete_user(user_id).await
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    AuthGuard(claims): AuthGuard,
    ValidJson(req): ValidJson<UpdateUserRequest>,
) -> Result<Json<UpdateUserResponse>, Error> {
    if claims.role != ROLE_ADMIN {
        if req.role.is_some()
            || req.deleted.is_some()
            || req.password.is_some()
            || req.user_id != claims.user_id
        {
            return Err(Error::Forbidden);
        }
    }
    let user = state.db.update_user(&req).await?;
    let rsp = UpdateUserResponse { user };
    Ok(Json(rsp))
}

async fn change_avatar(
    State(state): State<Arc<AppState>>,
    CookieGuard(claims): CookieGuard,
    mut multipart: Multipart,
) -> Result<Json<ChangeAvatarResponse>, Error> {
    let avatar = util::common::generate_avatar_name(claims.user_id);

    if let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(content_type) = field.content_type() {
            if !content_type.starts_with(IMAGE_KEY) {
                return Err(Error::BadRequest);
            }
        } else {
            return Err(Error::BadRequest);
        }

        let path = path::Path::new(&state.config.public_directory).join(&avatar[1..]);
        let data = field.bytes().await.unwrap();
        tokio::fs::write(&path, &data).await?;

        let old_avatar = state.db.change_avatar(claims.user_id, &avatar).await?;
        if old_avatar != DEFAULT_AVATAR {
            let path = path::Path::new(&state.config.public_directory).join(&old_avatar[1..]);
            if path.is_file() {
                tokio::fs::remove_file(&path).await?;
            }
        }
    } else {
        return Err(Error::BadRequest);
    }

    let rsp = ChangeAvatarResponse { avatar };
    Ok(Json(rsp))
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    AuthGuard(claims): AuthGuard,
    ValidJson(req): ValidJson<ChangePasswordRequest>,
) -> Result<(), Error> {
    state.db.change_password(claims.user_id, &req).await
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
    ValidQuery(req): ValidQuery<ListUsersRequest>,
) -> Result<Json<ListUsersResponse>, Error> {
    let (total, users) = state.db.list_users(&req.into()).await?;
    let rsp = ListUsersResponse { total, users };
    Ok(Json(rsp))
}

async fn find_user(
    State(state): State<Arc<AppState>>,
    AuthGuard(_): AuthGuard,
    Path(username): Path<String>,
) -> Result<Json<FindUserResponse>, Error> {
    if username.len() < 2 {
        return Err(Error::BadRequest);
    }
    let user = state.db.find_user(&username).await?.map(|u| u.into());
    let rsp = FindUserResponse { user };
    Ok(Json(rsp))
}
