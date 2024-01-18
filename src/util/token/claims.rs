use super::datetime;
use crate::store::User;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// The Claims of JWT token
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub user_id: i64,
    pub room_id: i64,
    pub role: String,
    pub sub: String,
    #[serde(with = "datetime")]
    pub exp: OffsetDateTime,
}

impl Claims {
    /// Create Claims from user_id, room_id, role and duration
    pub fn new(user_id: i64, room_id: i64, role: &str, sub: &str, duration: Duration) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            room_id,
            role: role.to_owned(),
            sub: sub.to_owned(),
            exp: OffsetDateTime::now_utc() + duration,
        }
    }

    /// Create Claims from user
    pub fn from_user(user: &User, sub: &str, duration: Duration) -> Self {
        Self::new(user.id, user.room_id, &user.role, sub, duration)
    }

    /// Create Claims from an old one
    pub fn from_claims(claims: Claims, sub: &str, duration: Duration) -> Self {
        Self::new(claims.user_id, claims.room_id, &claims.role, sub, duration)
    }
}
