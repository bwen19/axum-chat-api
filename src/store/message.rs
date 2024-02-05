//! Methods of Store for managing chat messages

use super::{model::MessageInfo, Store};
use crate::{
    api::NewMessageRequest,
    core::{
        constant::{DIVIDE_INTERVAL_MINUTE, MAX_CACHED_MESSAGE},
        Error,
    },
};
use redis::AsyncCommands;
use time::OffsetDateTime;

impl Store {
    pub async fn cache_message(
        &self,
        user_id: i64,
        req: NewMessageRequest,
    ) -> Result<MessageInfo, Error> {
        let user = self.get_user(user_id).await?;

        let mut con = self.client.get_async_connection().await?;
        let key = format!("room:{}", req.room_id);

        // check whether the last message was sent 5 minutes ago
        let last: Vec<String> = con.lrange(key.clone(), 0, 0).await?;
        let send_at = OffsetDateTime::now_utc();
        let divide = if let Some(last_msg_str) = last.first() {
            let msg = serde_json::from_str::<MessageInfo>(last_msg_str)?;
            if (send_at - msg.send_at).whole_minutes() > DIVIDE_INTERVAL_MINUTE {
                true
            } else {
                false
            }
        } else {
            true
        };

        // create new message
        let message = MessageInfo {
            room_id: req.room_id,
            sender_id: user.id,
            name: user.nickname,
            avatar: user.avatar,
            content: req.content,
            file_url: req.file_url,
            kind: req.kind,
            divide,
            send_at,
        };
        let msg_str = serde_json::to_string(&message)?;

        let total: isize = con.lpush(key.as_str(), msg_str).await?;
        if total > MAX_CACHED_MESSAGE {
            con.ltrim(key, 0, 19).await?;
        }

        Ok(message)
    }
}
