use super::user_socket::UserSocket;
use crate::error::AppResult;
use axum::extract::ws::{CloseFrame, Message};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use uuid::Uuid;

// ========================// Websocket Manager //======================== //

pub struct ChannelManager {
    users: Mutex<UserChannel>,
    rooms: Mutex<RoomChannel>,
    subscriptions: Mutex<Subscription>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(UserChannel::default()),
            rooms: Mutex::new(RoomChannel::default()),
            subscriptions: Mutex::new(Subscription::default()),
        }
    }

    /// Send close message to the client with same device
    pub async fn close(&self, uss: &UserSocket) -> AppResult<()> {
        let olders = {
            // get locks of HashMaps
            let users = self.users.lock().unwrap();
            users.old_senders(uss)
        };

        let msg = Message::Close(Some(CloseFrame {
            code: 1000,
            reason: "You have logged in elsewhere".into(),
        }));
        for sender in olders {
            sender.send(msg.clone()).await?;
        }

        Ok(())
    }

    /// Set conncetions between user's mpsc channel and room's broadcast channels
    pub fn connect(&self, uss: &UserSocket, room_ids: Vec<i64>, capacity: usize) {
        // get locks of HashMaps
        let mut users = self.users.lock().unwrap();
        let mut rooms = self.rooms.lock().unwrap();
        let mut subscriptions = self.subscriptions.lock().unwrap();

        let receivers = rooms.join_rooms(room_ids, capacity);
        subscriptions.subscribe_mult_rooms(&uss, receivers);
        users.insert(uss);
    }

    pub fn disconnect(&self, uss: &UserSocket) {
        // get locks of HashMaps
        let mut users = self.users.lock().unwrap();
        let mut rooms = self.rooms.lock().unwrap();
        let mut subscriptions = self.subscriptions.lock().unwrap();

        let room_ids = subscriptions.unsubscribe_rooms(&uss.socket_id);
        rooms.leave_rooms(room_ids);
        users.remove(uss.user_id, uss.socket_id);
    }

    pub fn join_room_mult(&self, room_id: i64, user_ids: &Vec<i64>, capacity: usize) {
        let users = self.users.lock().unwrap();
        let mut rooms = self.rooms.lock().unwrap();
        let mut subscriptions = self.subscriptions.lock().unwrap();

        let senders = users.mult_senders(user_ids);
        let receiver = rooms.join_room(room_id, senders.len() as i64, capacity);
        subscriptions.subscribe_room_mult(senders, room_id, receiver);
    }

    pub fn leave_room_mult(&self, room_id: i64, user_ids: Vec<i64>) {
        let users = self.users.lock().unwrap();
        let mut rooms = self.rooms.lock().unwrap();
        let mut subscriptions = self.subscriptions.lock().unwrap();

        let socket_ids = users.mult_sockets(user_ids);
        rooms.leave_room(room_id, socket_ids.len() as i64);
        subscriptions.unsubscribe_room_mult(socket_ids, room_id);
    }

    pub fn leave_room(&self, room_id: i64, user_id: i64) {
        self.leave_room_mult(room_id, vec![user_id]);
    }

    pub fn in_room(&self, socket_id: &Uuid, room_id: i64) -> bool {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.in_room(socket_id, room_id)
    }

    /// Send a message to users in one room
    pub fn send_to_room(&self, room_id: i64, msg: Message) {
        let sender = {
            let rooms = self.rooms.lock().unwrap();
            rooms.sender(room_id)
        };

        if let Some(sender) = sender {
            let _ = sender.send(msg);
        }
    }

    /// Send a message to mult users
    pub fn send_to_users(&self, user_ids: &Vec<i64>, msg: Message) {
        let room_ids = {
            let users = self.users.lock().unwrap();
            users.mult_rooms(user_ids)
        };

        let senders = {
            let rooms = self.rooms.lock().unwrap();
            rooms.senders(&room_ids)
        };

        for sender in senders {
            let _ = sender.send(msg.clone());
        }
    }

    pub fn send_to_user(&self, user_id: i64, msg: Message) {
        self.send_to_users(&vec![user_id], msg)
    }
}

// ========================// ChannelHolder //======================== //

struct ChannelHolder {
    socket_id: Uuid,
    user_agent: String,
    room_id: i64,
    tx: mpsc::Sender<Message>,
}

impl ChannelHolder {
    fn new(uss: &UserSocket) -> Self {
        Self {
            socket_id: uss.socket_id,
            user_agent: uss.user_agent.clone(),
            room_id: uss.room_id,
            tx: uss.tx.clone(),
        }
    }
}

// ========================// UserChannel //======================== //

/// mpsc channels for user's devices
#[derive(Default)]
pub struct UserChannel(HashMap<i64, Vec<ChannelHolder>>);

impl UserChannel {
    /// Insert a user's channel, and return channels which should be remove
    pub fn insert(&mut self, uss: &UserSocket) {
        let chh = ChannelHolder::new(uss);

        match self.0.entry(uss.user_id) {
            Entry::Occupied(mut o) => {
                let users = o.get_mut();
                users.push(chh);
            }
            Entry::Vacant(v) => {
                v.insert(vec![chh]);
            }
        }
    }

    /// Return senders with same agent as uss
    pub fn old_senders(&self, uss: &UserSocket) -> Vec<mpsc::Sender<Message>> {
        // collect senders that need to be removed
        let mut senders = Vec::new();

        if let Some(users) = self.0.get(&uss.user_id) {
            for user in users.iter() {
                if user.user_agent == uss.user_agent {
                    senders.push(user.tx.clone());
                }
            }
        }
        senders
    }

    /// Return all the socket_ids of a list of users
    pub fn mult_sockets(&self, user_ids: Vec<i64>) -> Vec<&Uuid> {
        let mut sockets = Vec::new();

        for user_id in user_ids {
            if let Some(chh) = self.0.get(&user_id) {
                for item in chh {
                    sockets.push(&item.socket_id);
                }
            }
        }

        sockets
    }

    /// Return all the senders of a list of users
    pub fn mult_senders(&self, user_ids: &Vec<i64>) -> Vec<(Uuid, mpsc::Sender<Message>)> {
        let mut senders = Vec::new();

        for user_id in user_ids {
            if let Some(chh) = self.0.get(user_id) {
                for item in chh {
                    senders.push((item.socket_id.clone(), item.tx.clone()));
                }
            }
        }

        senders
    }

    /// Return the personal rooms of a list of users
    pub fn mult_rooms(&self, user_ids: &Vec<i64>) -> Vec<i64> {
        let mut room_ids = Vec::new();

        for user_id in user_ids {
            if let Some(user) = self.0.get(user_id) {
                if let Some(chh) = user.first() {
                    room_ids.push(chh.room_id);
                }
            }
        }

        room_ids
    }

    /// Remove the user's channel
    pub fn remove(&mut self, user_id: i64, socket_id: Uuid) {
        if let Entry::Occupied(mut o) = self.0.entry(user_id) {
            let users = o.get_mut();

            users.retain(|uss| uss.socket_id != socket_id);
        }
    }
}

// ========================// RoomChannel //======================== //

/// Broadcast channels of chat rooms
#[derive(Default)]
pub struct RoomChannel(HashMap<i64, (broadcast::Sender<Message>, i64)>);

impl RoomChannel {
    /// Return a receiver of the given room and increase the number of subscribers
    ///
    /// This will create a new room if not exists
    pub fn join_room(
        &mut self,
        room_id: i64,
        num_members: i64,
        capacity: usize,
    ) -> broadcast::Receiver<Message> {
        match self.0.entry(room_id) {
            Entry::Occupied(mut o) => {
                let room = o.get_mut();
                room.1 += num_members;
                room.0.subscribe()
            }
            Entry::Vacant(v) => {
                let (tx, rx) = broadcast::channel(capacity);
                v.insert((tx, num_members));
                rx
            }
        }
    }

    /// Return receivers of given rooms and add subscribers
    ///
    /// This will create new rooms if not exists
    pub fn join_rooms(
        &mut self,
        room_ids: Vec<i64>,
        capacity: usize,
    ) -> Vec<(i64, broadcast::Receiver<Message>)> {
        room_ids
            .iter()
            .map(|&room_id| (room_id, self.join_room(room_id, 1, capacity)))
            .collect()
    }

    /// Decrease the number of subscribers in the room
    ///
    /// This will delete the room if the number is zero
    pub fn leave_room(&mut self, room_id: i64, num_members: i64) {
        if let Entry::Occupied(mut o) = self.0.entry(room_id) {
            let room = o.get_mut();
            room.1 -= num_members;
            if room.1 < num_members {
                o.remove();
            }
        }
    }

    pub fn leave_rooms(&mut self, room_ids: Vec<i64>) {
        for room_id in room_ids {
            self.leave_room(room_id, 1);
        }
    }

    /// Return a sender of the room
    pub fn sender(&self, room_id: i64) -> Option<broadcast::Sender<Message>> {
        if let Some((tx, _)) = self.0.get(&room_id) {
            Some(tx.clone())
        } else {
            None
        }
    }

    /// Return senders of a list of rooms
    pub fn senders(&self, room_ids: &Vec<i64>) -> Vec<broadcast::Sender<Message>> {
        let mut senders = Vec::new();

        for room_id in room_ids {
            if let Some((tx, _)) = self.0.get(room_id) {
                senders.push(tx.clone())
            }
        }

        senders
    }
}

// ========================// Subscription //======================== //

/// Rooms subscribed by a user
#[derive(Default)]
struct Subscription(HashMap<Uuid, HashMap<i64, JoinHandle<()>>>);

impl Subscription {
    /// set up a message transmission task between receiver and sender
    fn set_task(
        sender: mpsc::Sender<Message>,
        mut receiver: broadcast::Receiver<Message>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Ok(data) = receiver.recv().await {
                let _ = sender.send(data).await;
            }
        })
    }

    /// Subscribe multiple rooms and save the join handles
    pub fn subscribe_mult_rooms(
        &mut self,
        uss: &UserSocket,
        receivers: Vec<(i64, broadcast::Receiver<Message>)>,
    ) {
        // get the HashMap for storing JoinHandles
        let tasks = self.0.entry(uss.socket_id).or_insert(HashMap::new());

        // setup a new task for sending message
        for (room_id, receiver) in receivers {
            // clone a new sender of user's mpsc channel
            let sender = uss.tx.clone();

            // spawn a new task which receive message from a room's channel and send to user's channel
            let task = Self::set_task(sender, receiver);
            tasks.insert(room_id, task);
        }
    }

    /// Subscribe one room by one user
    pub fn subscribe_room(
        &mut self,
        socket_id: Uuid,
        sender: mpsc::Sender<Message>,
        room_id: i64,
        receiver: broadcast::Receiver<Message>,
    ) {
        // get the HashMap for storing JoinHandles
        let tasks = self.0.entry(socket_id).or_insert(HashMap::new());
        // set new task and insert
        let task = Self::set_task(sender, receiver);
        tasks.insert(room_id, task);
    }

    /// Subscribe one room by mult users
    pub fn subscribe_room_mult(
        &mut self,
        senders: Vec<(Uuid, mpsc::Sender<Message>)>,
        room_id: i64,
        receiver: broadcast::Receiver<Message>,
    ) {
        for (socket_id, sender) in senders {
            // spawn a new task which receive message from a room's channel and send to user's channel
            self.subscribe_room(socket_id, sender, room_id, receiver.resubscribe());
        }
    }

    /// Abort a subscription to a room and remove the task
    pub fn unsubscribe_room(&mut self, socket_id: &Uuid, room_id: i64) {
        if let Some(subs) = self.0.get_mut(socket_id) {
            if let Some(task) = subs.remove(&room_id) {
                task.abort();
            }
        }
    }

    pub fn unsubscribe_room_mult(&mut self, socket_ids: Vec<&Uuid>, room_id: i64) {
        for socket_id in socket_ids {
            self.unsubscribe_room(socket_id, room_id);
        }
    }

    /// Abort all the subscriptions by a user and remove
    pub fn unsubscribe_rooms(&mut self, socket_id: &Uuid) -> Vec<i64> {
        // a new vector to collect all room ids
        let mut room_ids = Vec::new();

        if let Some(subs) = self.0.get(socket_id) {
            for (&room_id, task) in subs {
                task.abort();
                room_ids.push(room_id);
            }
            self.0.remove(socket_id);
        }
        room_ids
    }

    pub fn in_room(&self, socket_id: &Uuid, room_id: i64) -> bool {
        if let Some(subs) = self.0.get(socket_id) {
            return subs.contains_key(&room_id);
        }
        false
    }
}
