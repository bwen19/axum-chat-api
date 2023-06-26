use crate::error::{AppError, AppResult};
use chrono::{DateTime, NaiveDateTime, Utc};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

/// Create random string with a given length
pub fn random_string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Generate random user avatar name
pub fn generate_avatar_name(user_id: i64) -> String {
    let ts = Utc::now().timestamp();
    format!("/avatar/img{}-{}{}", user_id, random_string(6), ts)
}

/// Generate random file path name
pub fn generate_file_name(file_name: &str) -> String {
    let ts = Utc::now().timestamp();
    format!("/share/{}{}-{}", random_string(6), ts, file_name)
}

/// Convert a timestamp to DateTime
pub fn convert_timestamp(ts: i64) -> AppResult<DateTime<Utc>> {
    let nt = NaiveDateTime::from_timestamp_opt(ts, 0).ok_or(AppError::TimeConversion)?;
    let dt = DateTime::from_utc(nt, Utc);
    Ok(dt)
}

// ========================// tests //======================== //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp() {
        let dt = Utc::now();
        let ts = dt.timestamp();
        let dt2 = convert_timestamp(ts).unwrap();

        assert_eq!(dt.timestamp(), dt2.timestamp());
    }
}
