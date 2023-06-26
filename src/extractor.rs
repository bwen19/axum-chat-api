use crate::{error::AppError, util::token::Claims, AppState, COOKIE_NAME};
use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, FromRequestParts, Query},
    headers::{authorization::Bearer, Authorization, UserAgent},
    http::{
        header::{HOST, USER_AGENT},
        request::Parts,
        Request,
    },
    Json, TypedHeader,
};
use axum_extra::extract::CookieJar;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use validator::Validate;

// ========================// ValidatedQuery //======================== //

/// Validate the values of query data from request
pub struct ValidatedQuery<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state).await?;
        value.validate()?;
        Ok(ValidatedQuery(value))
    }
}

// ========================// ValidatedJson //======================== //

/// Validate the values of json data from request body
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, B, T> FromRequest<S, B> for ValidatedJson<T>
where
    S: Send + Sync,
    B: Send + 'static,
    T: DeserializeOwned + Validate,
    Json<T>: FromRequest<S, B, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

// ========================// AuthGuard //======================== //

/// Verify JWT from request header and return the authenticated claims
pub struct AuthGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthGuard {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(token)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
        let claims = Claims::verify_token(token.token(), &state.config.jwt_secret)?;
        Ok(AuthGuard(claims))
    }
}

// ========================// AdminGuard //======================== //

/// Verify JWT and check whether the claims contain role of admin
pub struct AdminGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AdminGuard {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthGuard(claims) = AuthGuard::from_request_parts(parts, state).await?;

        if claims.is_admin() {
            Ok(AdminGuard(claims))
        } else {
            Err(AppError::Forbidden)
        }
    }
}

// ========================// CookieGuard //======================== //

/// Verify refresh token and return session
pub struct CookieGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for CookieGuard {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let cookie_jar = CookieJar::from_request_parts(parts, state).await?;
        let jwt_cookie = cookie_jar.get(COOKIE_NAME).ok_or(AppError::Unauthorized)?;

        let claims = Claims::verify_token(jwt_cookie.value(), &state.config.jwt_secret)?;
        let sess = state.db.get_session(&claims.id).await?;

        let TypedHeader(user_agent) =
            TypedHeader::<UserAgent>::from_request_parts(parts, state).await?;
        let user_agent = convert_agent(user_agent.to_string());

        if sess.id != claims.id
            || sess.user_id != claims.user_id
            || sess.refresh_token != jwt_cookie.value()
            || sess.user_agent != user_agent
        {
            Err(AppError::Unauthorized)
        } else {
            Ok(CookieGuard(claims))
        }
    }
}

// ========================// MetaData //======================== //

/// Get meta data from http header
pub struct MetaData {
    pub client_ip: String,
    pub user_agent: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for MetaData
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let client_ip = parts
            .headers
            .get(HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_owned();

        let user_agent = parts
            .headers
            .get(USER_AGENT)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("browser")
            .to_owned();

        // convert user_agent to shorter name
        let user_agent = convert_agent(user_agent);

        Ok(MetaData {
            client_ip,
            user_agent,
        })
    }
}

// ========================// SocketGuard //======================== //

/// Provide authentication for websocket connection
pub struct SocketGuard {
    pub user_id: i64,
    pub room_id: i64,
    pub role: String,
    pub user_agent: String,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for SocketGuard {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let cookie_jar = CookieJar::from_request_parts(parts, state).await?;
        let jwt_cookie = cookie_jar.get(COOKIE_NAME).ok_or(AppError::Unauthorized)?;

        let claims = Claims::verify_token(jwt_cookie.value(), &state.config.jwt_secret)?;
        let sess = state.db.get_session(&claims.id).await?;

        let TypedHeader(user_agent) =
            TypedHeader::<UserAgent>::from_request_parts(parts, state).await?;
        let user_agent = convert_agent(user_agent.to_string());

        if sess.id != claims.id
            || sess.user_id != claims.user_id
            || sess.refresh_token != jwt_cookie.value()
            || sess.user_agent != user_agent
        {
            Err(AppError::Unauthorized)
        } else {
            Ok(SocketGuard {
                user_id: claims.user_id,
                room_id: claims.room_id,
                role: claims.role,
                user_agent,
            })
        }
    }
}

// ========================// UTIL //======================== //

// Convert user agent to a related device
fn convert_agent(user_agent: String) -> String {
    let ua = user_agent.to_lowercase();

    let agent = if ua.contains("mobile") {
        "mobile"
    } else if ua.contains("desktop") {
        "desktop"
    } else if ua.contains("postman") {
        "postman"
    } else {
        "browser"
    };
    agent.to_owned()
}
