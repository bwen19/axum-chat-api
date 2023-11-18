use super::Client;
use axum::extract::ws::Message;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt,
};
use tokio::sync::mpsc;
use uuid::Uuid;

pub enum RoomAction {
    /// Send message to all clients in this room
    Send(Message),
    /// A new client join the room
    Join(Client),
    /// A client left the room
    Left(i64, Uuid),
    /// Add a user in the room
    Add(i64, HashMap<Uuid, mpsc::Sender<Message>>),
    /// Remove a user from the room
    Del(i64),
}

pub struct ChatRoom {
    rx: mpsc::Receiver<RoomAction>,
    txs: HashMap<i64, HashMap<Uuid, mpsc::Sender<Message>>>,
}

impl fmt::Display for ChatRoom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let num_users = self.txs.len();
        let mut num_clients = 0_usize;
        for clients in self.txs.values() {
            num_clients += clients.len();
        }
        write!(f, "users: {}, clients: {}", num_users, num_clients)
    }
}

impl ChatRoom {
    pub fn new(rx: mpsc::Receiver<RoomAction>) -> Self {
        Self {
            rx,
            txs: HashMap::default(),
        }
    }

    pub async fn serve(&mut self) {
        while let Some(action) = self.rx.recv().await {
            self.process(action).await;
        }
    }

    async fn process(&mut self, action: RoomAction) {
        match action {
            RoomAction::Send(msg) => {
                for senders in self.txs.values() {
                    for sender in senders.values() {
                        let _ = sender.send(msg.clone()).await;
                    }
                }
            }
            RoomAction::Join(client) => {
                match self.txs.entry(client.user_id()) {
                    Entry::Occupied(mut o) => {
                        let senders = o.get_mut();
                        senders.insert(client.id(), client.tx());
                    }
                    Entry::Vacant(v) => {
                        let mut senders = HashMap::new();
                        senders.insert(client.id(), client.tx());
                        v.insert(senders);
                    }
                };
            }
            RoomAction::Left(id, uid) => {
                if let Entry::Occupied(mut o) = self.txs.entry(id) {
                    let senders = o.get_mut();
                    senders.remove(&uid);
                }
            }
            RoomAction::Add(id, senders) => {
                self.txs.insert(id, senders);
            }
            RoomAction::Del(id) => {
                self.txs.remove(&id);
            }
        }
    }
}
