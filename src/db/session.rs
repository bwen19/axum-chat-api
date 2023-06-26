use super::{model::SessionEntity, Store};
use crate::{
    error::{AppResult, ResultExt},
    extractor::MetaData,
    util::{common::convert_timestamp, token::Claims},
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ========================// Session Store //======================== //

impl Store {
    /// Create a new session with the given params
    ///
    /// Update the session if the user_id coexists with the user_agent
    pub async fn create_session<'a>(&self, arg: &CreateSessionParam<'a>) -> AppResult<()> {
        let result = sqlx::query!(
            r#"
                UPDATE sessions
                SET
                    id = $1,
                    refresh_token = $2,
                    client_ip = $3,
                    expire_at = $4,
                    create_at = now()
                WHERE user_id = $5 AND user_agent = $6
            "#,
            arg.id,
            arg.refresh_token,
            arg.client_ip,
            arg.expire_at,
            arg.user_id,
            arg.user_agent,
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            return Ok(());
        }

        sqlx::query!(
            r#"
                INSERT INTO sessions (
                    id, user_id, refresh_token, client_ip, user_agent, expire_at
                )
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            arg.id,
            arg.user_id,
            arg.refresh_token,
            arg.client_ip,
            arg.user_agent,
            arg.expire_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a user's session. Used to log a user off
    pub async fn delete_session(&self, session_id: &Uuid, user_id: i64) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM sessions
                WHERE id = $1 AND user_id = $2
            "#,
            session_id,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Delete a list of sessions by IDs
    #[allow(dead_code)]
    pub async fn delete_sessions(&self, session_ids: &Vec<Uuid>) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM sessions
                WHERE id = ANY($1::uuid[])
            "#,
            session_ids
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Get a session by session ID
    pub async fn get_session(&self, session_id: &Uuid) -> AppResult<SessionEntity> {
        sqlx::query_as!(
            SessionEntity,
            r#"
                SELECT id, user_id, refresh_token, client_ip,
                    user_agent, expire_at, create_at
                FROM sessions WHERE id = $1
            "#,
            session_id
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()
    }
}

// ========================// Conversions //======================== //

pub struct CreateSessionParam<'a> {
    pub id: Uuid,
    pub user_id: i64,
    pub refresh_token: &'a String,
    pub client_ip: String,
    pub user_agent: String,
    pub expire_at: DateTime<Utc>,
}

impl<'a> CreateSessionParam<'a> {
    pub fn new(claims: Claims, md: MetaData, refresh_token: &'a String) -> AppResult<Self> {
        let expire_at = convert_timestamp(claims.exp)?;

        let arg = Self {
            id: claims.id,
            user_id: claims.user_id,
            refresh_token,
            client_ip: md.client_ip,
            user_agent: md.user_agent,
            expire_at,
        };
        Ok(arg)
    }
}
