use crate::{
    db::{model::UserInfo, session::CreateSessionParam},
    error::{AppError, AppResult},
    extractor::{CookieGuard, MetaData, ValidatedJson},
    util::{password::verify_password, token::Claims},
    AppState, COOKIE_NAME,
};
use axum::{extract::State, routing::post, Json, Router};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

// ========================// Auth Router //======================== //

/// Create auth router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/auto-login", post(auto_login))
        .route("/auth/renew-token", post(renew_token))
        .route("/auth/logout", post(logout))
}

// ========================// Register //======================== //

#[derive(Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub username: String,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub password: String,
    #[validate(length(min = 1, max = 50, message = "must be between 1 and 50 characters"))]
    pub code: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub user: UserInfo,
}

async fn register(
    State(state): State<Arc<AppState>>,
    ValidatedJson(req): ValidatedJson<RegisterRequest>,
) -> AppResult<Json<RegisterResponse>> {
    state.db.validate_invitation(&req.code).await?;
    let user = state.db.create_user(&req.into()).await?;

    Ok(Json(RegisterResponse { user }))
}

// ========================// Login //======================== //

#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 2, max = 50, message = "must be between 2 and 50 characters"))]
    pub username: String,
    #[validate(length(min = 6, max = 50, message = "must be between 6 and 50 characters"))]
    pub password: String,
    pub is_admin: Option<bool>,
    pub is_persisted: Option<bool>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub access_token: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    mata_data: MetaData,
    ValidatedJson(req): ValidatedJson<LoginRequest>,
) -> AppResult<(CookieJar, Json<LoginResponse>)> {
    // Get user entity by name
    let user = state
        .db
        .find_user(&req.username)
        .await?
        .ok_or(AppError::UserNotExist)?;

    // Check user status and password
    if user.deleted {
        return Err(AppError::Forbidden);
    } else if !verify_password(&req.password, &user.hashed_password)? {
        return Err(AppError::WrongPassword);
    } else if let Some(is_admin) = req.is_admin {
        if is_admin && user.role != "admin" {
            return Err(AppError::Forbidden);
        }
    }

    // Create access token with duration in minutes
    let duration = Duration::minutes(state.config.access_token_duration);
    let access_claims = Claims::new(&user, duration);
    let access_token = access_claims.create_token(&state.config.jwt_secret)?;

    // Create refresh token with duration in days
    let duration = Duration::days(state.config.refresh_token_duration);
    let refresh_claims = Claims::new(&user, duration);
    let refresh_token = refresh_claims.create_token(&state.config.jwt_secret)?;

    // Create session to save refresh token
    let arg = CreateSessionParam::new(refresh_claims, mata_data, &refresh_token)?;
    state.db.create_session(&arg).await?;

    // Create new cookie with the refresh token
    let mut cookie = Cookie::build(COOKIE_NAME, refresh_token)
        .path("/")
        .same_site(SameSite::Lax)
        .secure(true)
        .http_only(true)
        .finish();

    if let Some(is_persisted) = req.is_persisted {
        if is_persisted {
            cookie.make_permanent();
        }
    }

    // Return cookie and response
    let cookie_jar = cookie_jar.add(cookie);
    let rsp = Json(LoginResponse {
        user: user.into(),
        access_token,
    });

    Ok((cookie_jar, rsp))
}

// ========================// Auto Login //======================== //

#[derive(Deserialize)]
pub struct AutoLoginRequest {
    pub is_admin: Option<bool>,
}

async fn auto_login(
    State(state): State<Arc<AppState>>,
    CookieGuard(claims): CookieGuard,
    Json(req): Json<AutoLoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    // Get user info
    let user = state.db.get_user(claims.user_id).await?;

    if let Some(is_admin) = req.is_admin {
        if is_admin && user.role != "admin" {
            return Err(AppError::Forbidden);
        }
    }

    // Create new access token
    let duration = Duration::minutes(state.config.access_token_duration);
    let access_claims = Claims::new(&claims, duration);
    let access_token = access_claims.create_token(&state.config.jwt_secret)?;

    Ok(Json(LoginResponse { user, access_token }))
}

// ========================// Renew Token //======================== //

#[derive(Serialize)]
pub struct RenewTokenResponse {
    pub access_token: String,
}

async fn renew_token(
    State(state): State<Arc<AppState>>,
    CookieGuard(claims): CookieGuard,
) -> AppResult<Json<RenewTokenResponse>> {
    // Create new access token
    let duration = Duration::minutes(state.config.access_token_duration);
    let access_claims = Claims::new(&claims, duration);
    let access_token = access_claims.create_token(&state.config.jwt_secret)?;

    Ok(Json(RenewTokenResponse { access_token }))
}

// ========================// Logout //======================== //

async fn logout(State(state): State<Arc<AppState>>, cookie_jar: CookieJar) -> AppResult<CookieJar> {
    if let Some(jwt_cookie) = cookie_jar.get(COOKIE_NAME) {
        if let Ok(claims) = Claims::verify_token(jwt_cookie.value(), &state.config.jwt_secret) {
            let _ = state.db.delete_session(&claims.id, claims.user_id).await;
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
