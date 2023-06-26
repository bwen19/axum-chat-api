use crate::{
    db::model::User,
    error::{AppError, AppResult},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ========================// Claims //======================== //

/// The Claims of JWT token
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub user_id: i64,
    pub room_id: i64,
    pub role: String,
    pub exp: i64,
}

impl Claims {
    /// Create a new Claims from data with user_id, room_id and role
    ///
    /// Refreshable Claims has a longer expiration time
    pub fn new<T: WithUserData>(item: &T, duration: Duration) -> Self {
        let (user_id, room_id, role) = item.get_data();

        let id = Uuid::new_v4();
        let exp = Utc::now() + duration;

        Self {
            id,
            user_id,
            room_id,
            role,
            exp: exp.timestamp(),
        }
    }

    pub fn create_token(&self, secret: &String) -> AppResult<String> {
        encode::<Claims>(
            &Header::default(),
            self,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|_| AppError::TokenCreation)
    }

    pub fn verify_token(token: &str, secret: &String) -> AppResult<Self> {
        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| match e.into_kind() {
            ErrorKind::ExpiredSignature => AppError::ExpiredToken,
            _ => AppError::Unauthorized,
        })?;
        Ok(token_data.claims)
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

// ========================// WithUserData //======================== //

pub trait WithUserData {
    fn get_data(&self) -> (i64, i64, String);
}

impl WithUserData for Claims {
    fn get_data(&self) -> (i64, i64, String) {
        (self.user_id, self.room_id, self.role.clone())
    }
}

impl WithUserData for User {
    fn get_data(&self) -> (i64, i64, String) {
        (self.id, self.room_id, self.role.clone())
    }
}

// ========================// tests //======================== //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_token() {
        let secret = String::from("my_secret");

        let claims1 = Claims {
            id: Uuid::new_v4(),
            user_id: 120,
            room_id: 1,
            role: "ghost".to_owned(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
        };

        let token = claims1
            .create_token(&secret)
            .expect("failed to create token");

        let claims2 = Claims::verify_token(&token, &secret).expect("failed to decode token");

        assert_eq!(claims1.id, claims2.id);
        assert_eq!(claims1.user_id, claims2.user_id);
        assert_eq!(claims1.room_id, claims2.room_id);
        assert_eq!(claims1.role, claims2.role);
        assert_eq!(claims1.exp, claims2.exp);
    }

    #[test]
    #[should_panic(expected = "token is expired")]
    fn expired_token() {
        let secret = String::from("my_secret");

        let claims1 = Claims {
            id: Uuid::new_v4(),
            user_id: 120,
            room_id: 1,
            role: "ghost".to_owned(),
            exp: (Utc::now() + Duration::minutes(-2)).timestamp(),
        };

        let token = claims1
            .create_token(&secret)
            .expect("failed to create token");

        let _ = Claims::verify_token(&token, &secret).map_err(|e| match e {
            AppError::ExpiredToken => panic!("token is expired"),
            _ => e,
        });
    }

    #[test]
    #[should_panic(expected = "token is invalid")]
    fn invalid_token() {
        let secret = String::from("my_secret");

        let claims1 = Claims {
            id: Uuid::new_v4(),
            user_id: 120,
            room_id: 1,
            role: "ghost".to_owned(),
            exp: (Utc::now() + Duration::minutes(-2)).timestamp(),
        };

        let token = claims1
            .create_token(&secret)
            .expect("failed to create token");

        let secret = String::from("other_secret");
        let _ = Claims::verify_token(&token, &secret).map_err(|e| match e {
            AppError::ExpiredToken => e,
            _ => panic!("token is invalid"),
        });
    }
}
