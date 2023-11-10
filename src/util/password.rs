use crate::core::Error;
use argon2::{
    password_hash::{self, rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};

pub fn hash_password(password: &str) -> Result<String, Error> {
    let salt = SaltString::generate(&mut OsRng);

    let hashed_password = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| Error::Internal("failed to hash password".to_string()))?
        .to_string();
    Ok(hashed_password)
}

pub fn verify_password(password: &str, hashed_password: &str) -> Result<(), Error> {
    let hashed_password = PasswordHash::new(hashed_password)
        .map_err(|_| Error::Internal("invalid hash password".to_string()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &hashed_password)
        .map_err(|e| match e {
            password_hash::Error::Password => Error::WrongPassword,
            _ => Error::Internal("failed to verify password".to_string()),
        })
}

// ========================// tests //======================== //

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::common;

    #[test]
    fn test_password() {
        let password = common::random_string(7);
        let hashed_password = hash_password(&password).unwrap();

        assert!(verify_password(&password, &hashed_password).is_ok());
        assert!(verify_password("wrong", &hashed_password).is_err());
    }
}
