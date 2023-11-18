//! Methods of Store for managing auth sessions

use super::Store;
use crate::core::Error;
use redis::AsyncCommands;
use uuid::Uuid;

impl Store {
    pub async fn cache_session(
        &self,
        id: Uuid,
        refresh_token: &str,
        seconds: usize,
    ) -> Result<(), Error> {
        let mut con = self.client.get_async_connection().await?;
        con.set_ex(id.to_string(), refresh_token, seconds).await?;
        Ok(())
    }

    pub async fn get_session(&self, id: Uuid) -> Result<String, Error> {
        let mut con = self.client.get_async_connection().await?;
        let token: String = con.get(id.to_string()).await?;
        Ok(token)
    }

    pub async fn delete_session(&self, id: Uuid) -> Result<(), Error> {
        let mut con = self.client.get_async_connection().await?;
        con.del(id.to_string()).await?;
        Ok(())
    }
}
