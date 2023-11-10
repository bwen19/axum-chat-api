//! Handlers for authentication

use super::extractor::{CookieGuard, ValidJson};
use super::{AppState, AutoLoginRequest, LoginRequest, LoginResponse, RenewTokenResponse};
use crate::{
    core::{constant::COOKIE_NAME, Error},
    util::{password::verify_password, token::Claims},
};
use axum::{extract::State, routing::post, Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
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
    cookie_jar: CookieJar,
    ValidJson(req): ValidJson<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), Error> {
    // Get user by name
    let user = state
        .db
        .find_user(&req.username)
        .await?
        .ok_or(Error::UserNotExist)?;

    // Check user status
    if user.deleted {
        return Err(Error::Forbidden);
    } else if let Some(is_admin) = req.is_admin {
        if is_admin && user.role != "admin" {
            return Err(Error::Forbidden);
        }
    }

    // check password
    verify_password(&req.password, &user.hashed_password)?;

    // Create access token with duration in minutes
    let access_claims = Claims::from_user(&user, state.config.token_duration);
    let access_token = state.jwt.create(&access_claims)?;

    // Create refresh token with duration in days
    let refresh_claims = Claims::from_user(&user, state.config.session_duration);
    let refresh_token = state.jwt.create(&refresh_claims)?;

    // Create session to save refresh token
    state
        .db
        .create_session(
            refresh_claims.id,
            &refresh_token,
            state.config.session_seconds,
        )
        .await?;

    // Create new cookie with the refresh token
    let mut cookie = Cookie::build(COOKIE_NAME, refresh_token)
        .path("/")
        .same_site(SameSite::Lax)
        .secure(true)
        .http_only(true)
        .finish();

    cookie.set_expires(refresh_claims.exp);

    // Return cookie and response
    let cookie_jar = cookie_jar.add(cookie);
    let rsp = Json(LoginResponse {
        user: user.into(),
        access_token,
    });

    Ok((cookie_jar, rsp))
}

async fn auto_login(
    State(state): State<Arc<AppState>>,
    CookieGuard(claims): CookieGuard,
    Json(req): Json<AutoLoginRequest>,
) -> Result<Json<LoginResponse>, Error> {
    // Get user info
    let user = state.db.get_user(claims.user_id).await?;

    if let Some(is_admin) = req.is_admin {
        if is_admin && user.role != "admin" {
            return Err(Error::Forbidden);
        }
    }

    // Create new access token
    let access_claims = Claims::from_claims(claims, state.config.token_duration);
    let access_token = state.jwt.create(&access_claims)?;

    Ok(Json(LoginResponse { user, access_token }))
}

async fn renew_token(
    State(state): State<Arc<AppState>>,
    CookieGuard(claims): CookieGuard,
) -> Result<Json<RenewTokenResponse>, Error> {
    // Create new access token
    let access_claims = Claims::from_claims(claims, state.config.token_duration);
    let access_token = state.jwt.create(&access_claims)?;

    Ok(Json(RenewTokenResponse { access_token }))
}

async fn logout(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<CookieJar, Error> {
    if let Some(jwt_cookie) = cookie_jar.get(COOKIE_NAME) {
        if let Ok(claims) = state.jwt.verify(jwt_cookie.value()) {
            let _ = state.db.delete_session(claims.id).await;
        }
    }

    // Return cookie and response
    let mut cookie = Cookie::named(COOKIE_NAME);
    cookie.set_path("/");
    cookie.set_secure(true);
    cookie.set_http_only(true);
    let cookie_jar = cookie_jar.remove(cookie);

    Ok(cookie_jar)
}
