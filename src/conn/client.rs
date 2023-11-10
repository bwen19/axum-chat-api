use axum::extract::ws::Message;
use tokio::sync::mpsc;
use uuid::Uuid;

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

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn user_id(&self) -> i64 {
        self.user_id
    }

    pub fn room_id(&self) -> i64 {
        self.room_id
    }

    pub fn tx(&self) -> mpsc::Sender<Message> {
        self.tx.clone()
    }

    pub async fn send(&self, msg: Message) -> bool {
        self.tx.send(msg).await.is_ok()
    }
}
