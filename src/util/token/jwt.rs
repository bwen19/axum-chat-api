//! This module defines JwtToken which is used to create and verify token

use super::claims::Claims;
use crate::{core::Error, Config};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, DecodingKey, EncodingKey, Header, Validation,
};

/// Used to create and verify token
pub struct JwtToken {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtToken {
    pub fn new(config: &Config) -> Self {
        let secret = config.jwt_secret.as_bytes();
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
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

        let claims1 = Claims::new(120, 1, "user".to_owned(), Duration::seconds(1));
        let token = jwt.create(&claims1).expect("failed to create token");

        let claims2 = jwt.verify(&token).expect("failed to decode token");

        assert_eq!(claims1.id, claims2.id);
        assert_eq!(claims1.user_id, claims2.user_id);
        assert_eq!(claims1.room_id, claims2.room_id);
        assert_eq!(claims1.role, claims2.role);
        assert_eq!(claims1.exp.unix_timestamp(), claims2.exp.unix_timestamp());
    }

    #[test]
    fn expired_token() {
        let jwt = JwtToken::default();

        let claims1 = Claims::new(120, 1, "user".to_owned(), Duration::seconds(-61));
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

        let claims1 = Claims::new(120, 1, "user".to_owned(), Duration::minutes(10));
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
