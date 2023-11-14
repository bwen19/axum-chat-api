use crate::core::Error;
use axum::extract::ws::Message;
use tokio::sync::mpsc;
use uuid::Uuid;

/// A Client with a connection of user websocket
#[derive(Clone)]
pub struct Client {
    id: Uuid,
    user_id: i64,
    room_id: i64,
    tx: mpsc::Sender<Message>,
}

impl Client {
    pub fn new(user_id: i64, room_id: i64, tx: mpsc::Sender<Message>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            room_id,
            tx,
        }
    }

    /// Return id of the client
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Return user id of the client
    pub fn user_id(&self) -> i64 {
        self.user_id
    }

    /// Return user room id of the client
    pub fn room_id(&self) -> i64 {
        self.room_id
    }

    /// Return channel of the client
    pub fn tx(&self) -> mpsc::Sender<Message> {
        self.tx.clone()
    }

    /// Send message to the client
    pub async fn send(&self, msg: Message) -> Result<(), Error> {
        self.tx.send(msg).await?;
        Ok(())
    }
}
