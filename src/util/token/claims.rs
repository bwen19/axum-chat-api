use super::datetime;
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
    pub sub: bool,
    #[serde(with = "datetime")]
    pub exp: OffsetDateTime,
}

impl Claims {
    /// Create Claims from user_id, room_id, role and duration
    pub fn new(user_id: i64, room_id: i64, role: &str, sub: bool, duration: Duration) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            room_id,
            role: role.to_owned(),
            sub,
            exp: OffsetDateTime::now_utc() + duration,
        }
    }
}
