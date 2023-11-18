//! Methods of Store for managing chat messages

use super::{model::MessageInfo, Store};
use crate::{api::NewMessageRequest, core::Error};
use redis::AsyncCommands;
use time::OffsetDateTime;

impl Store {
    pub async fn cache_message(
        &self,
        user_id: i64,
        req: NewMessageRequest,
    ) -> Result<MessageInfo, Error> {
        let user = self.get_user(user_id).await?;

        let message = MessageInfo {
            room_id: req.room_id,
            sender_id: user.id,
            name: user.nickname,
            avatar: user.avatar,
            content: req.content,
            kind: req.kind,
            divide: false,
            send_at: OffsetDateTime::now_utc(),
        };
        let msg_str = serde_json::to_string(&message)?;

        let mut con = self.client.get_async_connection().await?;
        let key = format!("room:{}", req.room_id);
        let total: isize = con.lpush(key.as_str(), msg_str).await?;
        if total > 60 {
            con.ltrim(key, 0, 19).await?;
        }

        Ok(message)
    }
}
