use std::env;
use time::Duration;

#[derive(Debug)]
pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub public_directory: String,
    pub token_duration: Duration,
    pub session_duration: Duration,
    pub session_seconds: usize,
}

impl Config {
    pub fn from_env() -> Config {
        let server_addr = env::var("SERVER_ADDR").expect("SERVER_ADDR must be set");
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let public_directory = env::var("PUBLIC_DIRECTORY").expect("PUBLIC_DIRECTORY must be set");

        let token_duration: i64 = env::var("TOKEN_DURATION")
            .unwrap_or("15".to_owned())
            .parse()
            .expect("TOKEN_DURATION (minutes) must be set");
        let token_duration = Duration::minutes(token_duration);

        let session_duration: i64 = env::var("SESSION_DURATION")
            .unwrap_or("30".to_owned())
            .parse()
            .expect("SESSION_DURATION (days) must be set");
        let session_duration = Duration::days(session_duration);
        let session_seconds = session_duration
            .whole_seconds()
            .try_into()
            .expect("Conversion error of session seconds");

        Config {
            server_addr,
            database_url,
            redis_url,
            jwt_secret,
            public_directory,
            token_duration,
            session_duration,
            session_seconds,
        }
    }
}
