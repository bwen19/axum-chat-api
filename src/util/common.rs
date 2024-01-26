use rand::{distributions::Alphanumeric, thread_rng, Rng};
use time::OffsetDateTime;

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
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    format!("/avatar/img{}-{}{}", user_id, random_string(6), ts)
}

/// Generate random room cover name
pub fn generate_cover_name(room_id: i64) -> String {
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    format!("/cover/img{}-{}{}", room_id, random_string(6), ts)
}

/// Generate random file path name
pub fn generate_file_name(user_id: i64) -> String {
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    format!("/share/{}{}-{}", user_id, random_string(8), ts)
}
