use std::env;

// ========================// Config //======================== //

/// Configure of the App
#[derive(Debug)]
pub struct Config {
    pub ip: String,
    pub port: String,
    pub db_url: String,
    pub jwt_secret: String,
    pub public_directory: String,
    pub access_token_duration: i64,
    pub refresh_token_duration: i64,
    pub user_channel_capacity: usize,
    pub room_channel_capacity: usize,
}

impl Config {
    /// Initialize the Config from env
    pub fn from_env() -> Config {
        let ip = env::var("SERVER_IP").expect("failed to parse SERVER_IP");
        let port = env::var("SERVER_PORT").expect("failed to parse SERVER_PORT");
        let db_url = env::var("DATABASE_URL").expect("failed to parse DATABASE_URL");
        let jwt_secret = env::var("JWT_SECRET").expect("failed to parse JWT_SECRET");
        let public_directory =
            env::var("PUBLIC_DIRECTORY").expect("failed to parse PUBLIC_DIRECTORY");

        let access_token_duration: i64 = env::var("ACCESS_TOKEN_DURATION")
            .unwrap_or("15".to_owned())
            .parse()
            .expect("failed to parse ACCESS_TOKEN_DURATION");

        let refresh_token_duration: i64 = env::var("REFRESH_TOKEN_DURATION")
            .unwrap_or("30".to_owned())
            .parse()
            .expect("failed to parse REFRESH_TOKEN_DURATION");

        let user_channel_capacity = env::var("USER_CHANNEL_CAPACITY")
            .unwrap_or("100".to_owned())
            .parse()
            .expect("failed to parse USER_CHANNEL_CAPACITY");

        let room_channel_capacity = env::var("ROOM_CHANNEL_CAPACITY")
            .unwrap_or("100".to_owned())
            .parse()
            .expect("failed to parse ROOM_CHANNEL_CAPACITY");

        Config {
            ip,
            port,
            db_url,
            jwt_secret,
            public_directory,
            access_token_duration,
            refresh_token_duration,
            user_channel_capacity,
            room_channel_capacity,
        }
    }
}
