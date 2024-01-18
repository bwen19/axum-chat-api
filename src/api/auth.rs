//! Handlers for authentication

use super::{
    extractor::{RefreshGuard, ValidJson},
    AppState, AutoLoginRequest, AutoLoginResponse, LoginRequest, LoginResponse, RenewTokenResponse,
};
use crate::core::constant::{ACCESS_KEY, REFRESH_KEY, ROLE_ADMIN};
use crate::{
    core::Error,
    util::{password::verify_password, token::Claims},
};
use axum::{extract::State, routing::post, Json, Router};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/auto-login", post(auto_login))
        .route("/auth/renew-token", post(renew_token))
        .route("/auth/logout", post(logout))
}

async fn login(
    State(state): State<Arc<AppState>>,
    ValidJson(req): ValidJson<LoginRequest>,
) -> Result<Json<LoginResponse>, Error> {
    // get user by name
    let user = state
        .db
        .find_user(&req.username)
        .await?
        .ok_or(Error::NotFound)?;

    // verify user's status
    if user.deleted {
        return Err(Error::Forbidden);
    }
    if let Some(is_admin) = req.is_admin {
        if is_admin && user.role != ROLE_ADMIN {
            return Err(Error::Forbidden);
        }
    }

    // check password
    verify_password(&req.password, &user.hashed_password)?;

    // create access and refresh tokens
    let access_claims = Claims::from_user(&user, ACCESS_KEY, state.config.token_duration);
    let access_token = state.jwt.create(&access_claims)?;

    let refresh_claims = Claims::from_user(&user, REFRESH_KEY, state.config.session_duration);
    let refresh_token = state.jwt.create(&refresh_claims)?;

    // create session to save refresh token in redis
    state
        .db
        .cache_session(
            refresh_claims.id,
            &refresh_token,
            state.config.session_seconds,
        )
        .await?;

    // cache user info in redis
    let user = user.into();
    state.db.cache_user(&user).await?;

    // return cookie and response
    let rsp = LoginResponse {
        user,
        access_token,
        refresh_token,
    };
    Ok(Json(rsp))
}

async fn auto_login(
    State(state): State<Arc<AppState>>,
    RefreshGuard(claims): RefreshGuard,
    Json(req): Json<AutoLoginRequest>,
) -> Result<Json<AutoLoginResponse>, Error> {
    // get user info
    let user = state.db.get_user(claims.user_id).await?;

    // verify user status
    if user.deleted {
        return Err(Error::Forbidden);
    }
    if let Some(is_admin) = req.is_admin {
        if is_admin && user.role != ROLE_ADMIN {
            return Err(Error::Forbidden);
        }
    }

    // create access token
    let access_claims = Claims::from_claims(claims, ACCESS_KEY, state.config.token_duration);
    let access_token = state.jwt.create(&access_claims)?;

    let rsp = AutoLoginResponse { user, access_token };
    Ok(Json(rsp))
}

async fn renew_token(
    State(state): State<Arc<AppState>>,
    RefreshGuard(claims): RefreshGuard,
) -> Result<Json<RenewTokenResponse>, Error> {
    // create access token
    let access_claims = Claims::from_claims(claims, ACCESS_KEY, state.config.token_duration);
    let access_token = state.jwt.create(&access_claims)?;

    let rsp = RenewTokenResponse { access_token };
    Ok(Json(rsp))
}

async fn logout(
    State(state): State<Arc<AppState>>,
    RefreshGuard(claims): RefreshGuard,
) -> Result<(), Error> {
    // verify token from header and delete the session
    let _ = state.db.delete_session(claims.id).await;

    Ok(())
}
