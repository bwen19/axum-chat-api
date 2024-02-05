use std::env;

#[derive(Debug)]
pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub public_directory: String,
    pub access_token_minutes: i64,
    pub refresh_token_days: i64,
    pub session_seconds: usize,
}

impl Config {
    pub fn from_env() -> Config {
        let server_addr = env::var("SERVER_ADDR").expect("SERVER_ADDR must be set");
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let public_directory = env::var("PUBLIC_DIRECTORY").expect("PUBLIC_DIRECTORY must be set");

        let access_token_minutes: i64 = env::var("ACCESS_TOKEN_MINUTES")
            .unwrap_or("15".to_owned())
            .parse()
            .expect("ACCESS_TOKEN_MINUTES must be set");

        let refresh_token_days: i64 = env::var("REFRESH_TOKEN_DAYS")
            .unwrap_or("30".to_owned())
            .parse()
            .expect("REFRESH_TOKEN_DAYS must be set");

        let session_seconds = (refresh_token_days * 24 * 60 * 60)
            .try_into()
            .expect("Conversion error of session days");

        Config {
            server_addr,
            database_url,
            redis_url,
            jwt_secret,
            public_directory,
            access_token_minutes,
            refresh_token_days,
            session_seconds,
        }
    }
}
