//! Handlers for authentication

use super::{
    extractor::ValidJson, AppState, AutoLoginRequest, LoginRequest, LoginResponse, LogoutRequest,
    RenewTokenRequest, RenewTokenResponse,
};
use crate::{
    core::{constant::ROLE_ADMIN, Error},
    util::{
        password::verify_password,
        token::{Claims, JwtTokenPair},
    },
};
use axum::{extract::State, routing::post, Json, Router};
use std::sync::Arc;
use time::OffsetDateTime;

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

    // verify password
    verify_password(&req.password, &user.hashed_password)?;

    // create access and refresh tokens pair
    let JwtTokenPair(access_token, refresh_token, session_id, expire_at) =
        state.jwt.create_pair((&user).into())?;

    // create session to save refresh token in redis
    state
        .db
        .cache_session(session_id, &refresh_token, state.config.session_seconds)
        .await?;

    // cache user info in redis
    let user = user.into();
    state.db.cache_user(&user).await?;

    // return cookie and response
    let rsp = LoginResponse {
        user,
        access_token,
        refresh_token,
        expire_at,
    };
    Ok(Json(rsp))
}

async fn auto_login(
    State(state): State<Arc<AppState>>,
    ValidJson(req): ValidJson<AutoLoginRequest>,
) -> Result<Json<LoginResponse>, Error> {
    // parse claims from refresh token
    let claims = extract_claims(&state, req.refresh_token).await?;

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

    // generate new token pair
    let (access_token, refresh_token, expire_at) = renew_token_pair(&state, &claims).await?;

    let rsp = LoginResponse {
        user,
        access_token,
        refresh_token,
        expire_at,
    };
    Ok(Json(rsp))
}

async fn renew_token(
    State(state): State<Arc<AppState>>,
    ValidJson(req): ValidJson<RenewTokenRequest>,
) -> Result<Json<RenewTokenResponse>, Error> {
    // parse claims from refresh token
    let claims = extract_claims(&state, req.refresh_token).await?;

    // generate new token pair
    let (access_token, refresh_token, expire_at) = renew_token_pair(&state, &claims).await?;

    let rsp = RenewTokenResponse {
        access_token,
        refresh_token,
        expire_at,
    };
    Ok(Json(rsp))
}

async fn logout(
    State(state): State<Arc<AppState>>,
    ValidJson(req): ValidJson<LogoutRequest>,
) -> Result<(), Error> {
    let claims = extract_claims(&state, req.refresh_token).await?;
    let _ = state.db.delete_session(claims.id).await;

    Ok(())
}

async fn extract_claims(state: &Arc<AppState>, refresh_token: String) -> Result<Claims, Error> {
    // verify refresh token
    let claims = state
        .jwt
        .verify(&refresh_token)
        .map_err(|_| Error::Unauthorized)?;

    if !claims.sub {
        return Err(Error::Forbidden);
    }

    // check whether refresh_token exists in session
    let refresh_token = state
        .db
        .get_session(claims.id)
        .await
        .map_err(|_| Error::Unauthorized)?;
    if refresh_token != refresh_token {
        return Err(Error::Unauthorized);
    }

    Ok(claims)
}

async fn renew_token_pair(
    state: &Arc<AppState>,
    claims: &Claims,
) -> Result<(String, String, OffsetDateTime), Error> {
    // generate new token pair
    let JwtTokenPair(access_token, refresh_token, session_id, expire_at) =
        state.jwt.renew_pair(claims)?;

    // create new session
    state
        .db
        .cache_session(session_id, &refresh_token, state.config.session_seconds)
        .await?;

    // delete old session
    state.db.delete_session(claims.id).await?;

    Ok((access_token, refresh_token, expire_at))
}
