//! Defines result extractors

use super::error::Error;

/// A helper trait for easily converting SqlxError into App errors
pub trait ResultExt<T> {
    /// If self contains a database constraint error with the given name,
    /// transform the error
    fn on_constraint(self, name: &str) -> Result<T, Error>;

    /// Handle not found error by fetching exactly one
    fn not_found(self) -> Result<T, Error>;
}

impl<T> ResultExt<T> for Result<T, sqlx::Error> {
    fn on_constraint(self, name: &str) -> Result<T, Error> {
        self.map_err(|e| match e {
            sqlx::Error::Database(dbe) if dbe.constraint() == Some(name) => {
                Error::UniqueConstraint(name.to_string())
            }
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::Sqlx(e),
        })
    }

    fn not_found(self) -> Result<T, Error> {
        self.map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::Sqlx(e),
        })
    }
}
