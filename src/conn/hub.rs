use crate::core::Error;

use super::{client::Client, state::HubState};
use axum::extract::ws::Message;
use tokio::sync::RwLock;

// ============================== // Hub // ============================== //

#[derive(Default)]
pub struct Hub {
    inner: RwLock<HubState>,
}

impl Hub {
    pub async fn broadcast(&self, room_id: i64, msg: Message) -> Result<(), Error> {
        let inner = self.inner.read().await;
        inner.send(room_id, msg).await
    }

    pub async fn is_user_in(&self, user_id: i64, room_id: i64) -> bool {
        let inner = self.inner.read().await;
        inner.is_user_in(user_id, room_id)
    }

    pub async fn notify(&self, users: &Vec<i64>, msg: Message) -> Result<(), Error> {
        let inner = self.inner.read().await;
        for &user_id in users.iter() {
            if let Some(room_id) = inner.user_room(user_id) {
                inner.send(room_id, msg.clone()).await?;
            }
        }
        Ok(())
    }

    pub async fn tell(&self, user_id: i64, msg: Message) -> Result<(), Error> {
        let inner = self.inner.read().await;
        if let Some(room_id) = inner.user_room(user_id) {
            inner.send(room_id, msg).await?;
        }
        Ok(())
    }

    pub async fn connect(&self, client: &Client, rooms: Vec<i64>) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        inner.insert_client(client, rooms).await
    }

    pub async fn disconnect(&self, client: &Client) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        inner.remove_client(client).await
    }

    pub async fn add_members(&self, room_id: i64, users: &Vec<i64>) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        inner.add_members(room_id, users).await
    }

    pub async fn remove_members(&self, room_id: i64, users: &Vec<i64>) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        inner.remove_members(room_id, users).await
    }
    pub async fn remove_member(&self, room_id: i64, user_id: i64) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        inner.remove_members(room_id, &vec![user_id]).await
    }
}
