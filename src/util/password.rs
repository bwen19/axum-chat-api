use crate::core::Error;
use argon2::{
    password_hash::{self, rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};

pub fn hash_password(password: &str) -> Result<String, Error> {
    let salt = SaltString::generate(&mut OsRng);

    let hashed_password = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| Error::Argon2)?
        .to_string();
    Ok(hashed_password)
}

pub fn verify_password(password: &str, hashed_password: &str) -> Result<(), Error> {
    let hashed_password = PasswordHash::new(hashed_password).map_err(|_| Error::Argon2)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &hashed_password)
        .map_err(|e| match e {
            password_hash::Error::Password => Error::InvalidPassword,
            _ => Error::Argon2,
        })
}

// ============================== // tests // ============================== //

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::common;

    #[test]
    fn test_password() {
        let password = common::random_string(7);
        let hashed_password = hash_password(&password).unwrap();

        assert!(verify_password(&password, &hashed_password).is_ok());

        let res = verify_password("wrong", &hashed_password);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            Error::InvalidPassword.to_string()
        );
    }
}
