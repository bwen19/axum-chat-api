//! This module defines JwtToken which is used to create and verify token

use super::claims::Claims;
use crate::{
    core::{constant::MAX_AHEAD_MINUTE, Error},
    store::User,
    Config,
};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, DecodingKey, EncodingKey, Header, Validation,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub struct JwtTokenPair(pub String, pub String, pub Uuid, pub OffsetDateTime);

/// Used to create and verify token
pub struct JwtToken {
    encoding: EncodingKey,
    decoding: DecodingKey,
    access_token_duration: Duration,
    refresh_token_duration: Duration,
    ahead_duration: Duration,
}

impl JwtToken {
    pub fn new(config: &Config) -> Self {
        let secret = config.jwt_secret.as_bytes();
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            access_token_duration: Duration::minutes(config.access_token_minutes),
            refresh_token_duration: Duration::days(config.refresh_token_days),
            ahead_duration: Duration::minutes(MAX_AHEAD_MINUTE),
        }
    }

    pub fn create_pair(&self, user: &User) -> Result<JwtTokenPair, Error> {
        let access_claims = Claims::new(
            user.id,
            user.room_id,
            &user.role,
            false,
            self.access_token_duration,
        );
        let access_token = self.create(&access_claims)?;

        let refresh_claims = Claims::new(
            user.id,
            user.room_id,
            &user.role,
            true,
            self.refresh_token_duration,
        );
        let refresh_token = self.create(&refresh_claims)?;

        Ok(JwtTokenPair(
            access_token,
            refresh_token,
            refresh_claims.id,
            access_claims.exp - self.ahead_duration,
        ))
    }

    pub fn renew_pair(&self, claims: &Claims) -> Result<JwtTokenPair, Error> {
        let access_claims = Claims::new(
            claims.user_id,
            claims.room_id,
            &claims.role,
            false,
            self.access_token_duration,
        );
        let access_token = self.create(&access_claims)?;

        let refresh_claims = Claims::new(
            claims.user_id,
            claims.room_id,
            &claims.role,
            true,
            self.refresh_token_duration,
        );
        let refresh_token = self.create(&refresh_claims)?;

        Ok(JwtTokenPair(
            access_token,
            refresh_token,
            refresh_claims.id,
            access_claims.exp - self.ahead_duration,
        ))
    }

    pub fn create(&self, claims: &Claims) -> Result<String, Error> {
        let token = encode::<Claims>(&Header::default(), claims, &self.encoding)?;
        Ok(token)
    }

    pub fn verify(&self, token: &str) -> Result<Claims, Error> {
        let data = decode::<Claims>(&token, &self.decoding, &Validation::default()).map_err(
            |e| match e.into_kind() {
                ErrorKind::ExpiredSignature => Error::ExpiredToken,
                _ => Error::Unauthorized,
            },
        )?;

        Ok(data.claims)
    }
}

impl Default for JwtToken {
    fn default() -> Self {
        let secret = b"secret";
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            access_token_duration: Duration::minutes(15),
            refresh_token_duration: Duration::days(30),
            ahead_duration: Duration::minutes(3),
        }
    }
}

// ============================== // tests // ============================== //

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn verify_token() {
        let jwt = JwtToken::default();

        let claims1 = Claims::new(120, 1, "user", false, Duration::seconds(1));
        let token = jwt.create(&claims1).expect("failed to create token");

        let claims2 = jwt.verify(&token).expect("failed to decode token");

        assert_eq!(claims1.id, claims2.id);
        assert_eq!(claims1.user_id, claims2.user_id);
        assert_eq!(claims1.room_id, claims2.room_id);
        assert_eq!(claims1.role, claims2.role);
        assert_eq!(claims1.sub, claims2.sub);
        assert_eq!(claims1.exp.unix_timestamp(), claims2.exp.unix_timestamp());
    }

    #[test]
    fn expired_token() {
        let jwt = JwtToken::default();

        let claims1 = Claims::new(120, 1, "user", false, Duration::seconds(-61));
        let token = jwt.create(&claims1).expect("failed to create token");

        let res = jwt.verify(&token);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            Error::ExpiredToken.to_string()
        );
    }

    #[test]
    fn invalid_token() {
        let jwt = JwtToken::default();

        let claims1 = Claims::new(120, 1, "user", false, Duration::minutes(10));
        let mut token = jwt.create(&claims1).expect("failed to create token");

        let start = token.len() - 3;
        token.replace_range(start.., "098");

        let res = jwt.verify(&token);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            Error::Unauthorized.to_string()
        );
    }
}
