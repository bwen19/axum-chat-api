use super::client::Client;
use super::room::{ChatRoom, RoomAction};
use crate::core::constant::CHAN_CAPACITY;
use crate::core::Error;
use axum::extract::ws::Message;
// use std::collections::hash_map::Entry;
use std::collections::{hash_map::Entry, HashMap, HashSet};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Default)]
pub struct HubState {
    users: HashMap<i64, UserState>,
    rooms: HashMap<i64, RoomState>,
}

impl HubState {
    /// Send message to all clients in the room
    pub async fn send(&self, room_id: i64, msg: Message) -> Result<(), Error> {
        if let Some(room) = self.rooms.get(&room_id) {
            room.tx.send(RoomAction::Send(msg)).await?
        }
        Ok(())
    }

    pub async fn insert_client(&mut self, client: &Client, rooms: Vec<i64>) -> Result<(), Error> {
        for &room_id in rooms.iter() {
            let tx = self.get_room_chan(room_id);

            tx.send(RoomAction::Join(client.clone())).await?;
        }

        match self.users.entry(client.user_id()) {
            Entry::Occupied(mut o) => {
                let us = o.get_mut();
                us.txs.insert(client.id(), client.tx());
            }
            Entry::Vacant(v) => {
                let us = UserState::new(client, rooms);
                v.insert(us);
            }
        };
        Ok(())
    }

    pub async fn remove_client(&mut self, client: &Client) -> Result<(), Error> {
        if let Some(us) = self.users.get_mut(&client.user_id()) {
            for room_id in us.rooms.iter() {
                if let Some(room) = self.rooms.get(room_id) {
                    room.tx
                        .send(RoomAction::Left(client.user_id(), client.id()))
                        .await?;
                }
            }

            us.txs.remove(&client.id());
            if us.txs.is_empty() {
                self.users.remove(&client.user_id());
            }
        }
        Ok(())
    }

    pub async fn add_members(&mut self, room_id: i64, users: &Vec<i64>) -> Result<(), Error> {
        let tx = self.get_room_chan(room_id);

        for user_id in users.iter() {
            if let Some(us) = self.users.get_mut(user_id) {
                tx.send(RoomAction::Add(*user_id, us.txs.clone())).await?;
                us.rooms.insert(room_id);
            }
        }
        Ok(())
    }

    pub async fn remove_members(&mut self, room_id: i64, users: &Vec<i64>) -> Result<(), Error> {
        let tx = self.get_room_chan(room_id);

        for user_id in users.iter() {
            if let Some(us) = self.users.get_mut(user_id) {
                tx.send(RoomAction::Del(*user_id)).await?;
                us.rooms.remove(&room_id);
            }
        }
        Ok(())
    }

    pub fn is_user_in(&self, user_id: i64, room_id: i64) -> bool {
        if let Some(us) = self.users.get(&user_id) {
            us.rooms.contains(&room_id)
        } else {
            false
        }
    }

    pub fn user_room(&self, user_id: i64) -> Option<i64> {
        self.users.get(&user_id).map(|u| u.user_room)
    }

    fn get_room_chan(&mut self, room_id: i64) -> mpsc::Sender<RoomAction> {
        self.rooms
            .get(&room_id)
            .map(|r| r.tx.clone())
            .unwrap_or(self.create_room(room_id))
    }

    fn create_room(&mut self, room_id: i64) -> mpsc::Sender<RoomAction> {
        let (tx, rx) = mpsc::channel(CHAN_CAPACITY);
        let mut chat_room = ChatRoom::new(rx);

        let task = tokio::spawn(async move {
            chat_room.serve().await;
        });

        let room = RoomState::new(tx.clone(), task);
        self.rooms.insert(room_id, room);

        tx
    }
}

struct UserState {
    txs: HashMap<Uuid, mpsc::Sender<Message>>,
    user_room: i64,      // user room id
    rooms: HashSet<i64>, // joined room list
}

impl UserState {
    fn new(client: &Client, rooms: Vec<i64>) -> Self {
        let mut txs = HashMap::new();
        txs.insert(client.id(), client.tx());

        let rooms = rooms.into_iter().collect();

        Self {
            txs,
            user_room: client.room_id(),
            rooms,
        }
    }
}

#[allow(dead_code)]
struct RoomState {
    tx: mpsc::Sender<RoomAction>,
    task: JoinHandle<()>,
}

impl RoomState {
    fn new(tx: mpsc::Sender<RoomAction>, task: JoinHandle<()>) -> Self {
        Self { tx, task }
    }
}
