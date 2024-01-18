//! Defines the extractors used by different web services.

use super::AppState;
use crate::core::constant::{ACCESS_KEY, REFRESH_KEY, ROLE_ADMIN};
use crate::{core::Error, util::token::Claims};
use axum::{
    async_trait,
    extract::{FromRequest, FromRequestParts, Query, Request},
    http::header::{AUTHORIZATION, SEC_WEBSOCKET_PROTOCOL},
    http::request::Parts,
    Json,
};
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
impl<S, T> FromRequest<S> for ValidJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
    // Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state).await?;
        data.validate()?;
        Ok(ValidJson(data))
    }
}

struct BearerToken(String);

#[async_trait]
impl<S> FromRequestParts<S> for BearerToken
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(Error::Unauthorized)?;

        match authorization.split_once(' ') {
            Some((name, content)) if name == "Bearer" => Ok(BearerToken(content.to_owned())),
            _ => Err(Error::Unauthorized),
        }
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
        let BearerToken(token) = BearerToken::from_request_parts(parts, state).await?;
        let claims = state.jwt.verify(&token)?;

        if claims.sub == ACCESS_KEY {
            Ok(AuthGuard(claims))
        } else {
            Err(Error::Unauthorized)
        }
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

/// Extracts the refresh JWT Claims from the request header.
pub struct RefreshGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for RefreshGuard {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let BearerToken(token) = BearerToken::from_request_parts(parts, state).await?;
        let claims = state.jwt.verify(&token).map_err(|_| Error::Unauthorized)?;

        let refresh_token = state
            .db
            .get_session(claims.id)
            .await
            .map_err(|_| Error::Unauthorized)?;

        if claims.sub == REFRESH_KEY && refresh_token == token {
            Ok(RefreshGuard(claims))
        } else {
            Err(Error::Unauthorized)
        }
    }
}

/// Extracts the JWT from the request SEC_WEBSOCKET_PROTOCOL.
pub struct WsGuard(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for WsGuard {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let sec_websocket_protocol = parts
            .headers
            .get(SEC_WEBSOCKET_PROTOCOL)
            .and_then(|v| v.to_str().ok());
        // .ok_or(Error::Unauthorized)?;

        let (_, token) = sec_websocket_protocol
            .and_then(|v| v.split_once(","))
            .ok_or(Error::Unauthorized)?;

        let claims = state.jwt.verify(token.trim())?;
        if claims.sub == ACCESS_KEY {
            return Ok(WsGuard(claims));
        }

        Err(Error::Unauthorized)
    }
}
