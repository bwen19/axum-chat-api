use crate::error::{AppError, AppResult};
use bcrypt::DEFAULT_COST;

/// Create hash by bcrypt
pub fn hash_password(password: &str) -> AppResult<String> {
    bcrypt::hash(password, DEFAULT_COST).map_err(AppError::Bcrypt)
}

/// Verify the hash password using bcrypt
pub fn verify_password(password: &str, hashed_password: &str) -> AppResult<bool> {
    bcrypt::verify(password, hashed_password).map_err(AppError::Bcrypt)
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

        assert!(verify_password(&password, &hashed_password).unwrap());
        assert!(!verify_password("wrong", &hashed_password).unwrap());
    }
}
