//! Defines the extractors used by different web services.

use super::AppState;
use crate::core::constant::{COOKIE_NAME, ROLE_ADMIN};
use crate::{core::Error, util::token::Claims};
use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, FromRequestParts, Query},
    headers::{authorization::Bearer, Authorization},
    http::{request::Parts, Request},
    Json, TypedHeader,
};
use axum_extra::extract::CookieJar;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use validator::Validate;

/// Extracts the Query data from request url.
///
/// Validate the values of Query data.
pub struct ValidQuery<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(data) = Query::<T>::from_request_parts(parts, state).await?;
        data.validate()?;
        Ok(ValidQuery(data))
    }
}

/// Extracts the Json data from request body.
///
/// Validate the values of Json data.
pub struct ValidJson<T>(pub T);

#[async_trait]
impl<S, B, T> FromRequest<S, B> for ValidJson<T>
where
    S: Send + Sync,
    B: Send + 'static,
    T: DeserializeOwned + Validate,
    Json<T>: FromRequest<S, B, Rejection = JsonRejection>,
{
    type Rejection = Error;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state).await?;
        data.validate()?;
        Ok(ValidJson(data))
    }
}

/// Extracts the JWT from the request header.
pub struct AuthGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthGuard {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(token)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
        let claims = state.jwt.verify(token.token())?;
        Ok(AuthGuard(claims))
    }
}

/// Extractor used to check that :
///
/// 1. The user is authenticated.
/// 2. The user has at least admin roles in database.
pub struct AdminGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AdminGuard {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthGuard(claims) = AuthGuard::from_request_parts(parts, state).await?;

        if claims.role == ROLE_ADMIN {
            Ok(AdminGuard(claims))
        } else {
            Err(Error::Forbidden)
        }
    }
}

/// Extractor used to check token from cookie :
///
/// 1. The user is authenticated.
/// 2. The session has been saved in redis.
pub struct CookieGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for CookieGuard {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let cookie_jar = CookieJar::from_request_parts(parts, state).await?;

        let jwt_cookie = cookie_jar.get(COOKIE_NAME).ok_or(Error::Unauthorized)?;
        let claims = state.jwt.verify(jwt_cookie.value())?;

        let refresh_token = state.db.get_session(claims.id).await?;
        if refresh_token != jwt_cookie.value() {
            Err(Error::Unauthorized)
        } else {
            Ok(CookieGuard(claims))
        }
    }
}
