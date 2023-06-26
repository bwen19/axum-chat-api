use super::{model::MessageInfo, Store};
use crate::{
    api::message::{NewMessageRequest, NewMessageResponse},
    error::{AppResult, ResultExt},
};
use chrono::{DateTime, Utc};

// ========================// Message Store //======================== //

impl Store {
    /// Store new message in the database
    pub async fn create_message(
        &self,
        sender_id: i64,
        arg: &NewMessageRequest,
    ) -> AppResult<NewMessageResponse> {
        let result = sqlx::query_as!(
            MessageInfoRow,
            r#"
                WITH insert_cte AS (
                    INSERT INTO messages (room_id, sender_id, content, kind)
                    VALUES ($1, $2, $3, $4)
                    RETURNING id, room_id, sender_id, content, kind, send_at
                )
                SELECT ic.id, ic.room_id, sender_id, content, kind, ic.send_at,
                    nickname AS sender_name, avatar AS sender_avatar
                FROM insert_cte AS ic
                INNER JOIN users AS u ON ic.sender_id = u.id
            "#,
            arg.room_id,
            sender_id,
            arg.content,
            arg.kind,
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()?;

        Ok(result.into())
    }
}

// ========================// Conversions //======================== //

struct MessageInfoRow {
    id: i64,
    room_id: i64,
    sender_id: i64,
    sender_name: String,
    sender_avatar: String,
    content: String,
    kind: String,
    send_at: DateTime<Utc>,
}

impl From<MessageInfoRow> for NewMessageResponse {
    fn from(v: MessageInfoRow) -> Self {
        Self {
            room_id: v.room_id,
            message: MessageInfo {
                id: v.id,
                sid: v.sender_id,
                name: v.sender_name,
                avatar: v.sender_avatar,
                content: v.content,
                kind: v.kind,
                divide: false,
                send_at: v.send_at,
            },
        }
    }
}
