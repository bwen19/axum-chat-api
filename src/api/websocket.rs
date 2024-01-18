//! Handlers for websocket

use super::{extractor::WsGuard, AppState};
use crate::api::event::ClientEvent;
use crate::core::constant::{CHAN_CAPACITY, WS_SUB_PROTOCOL_KEY};
use crate::{conn::Client, util::token::Claims};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::{response::IntoResponse, routing::get, Router};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    WsGuard(claims): WsGuard,
) -> impl IntoResponse {
    ws.protocols([WS_SUB_PROTOCOL_KEY])
        .on_upgrade(move |socket| websocket(socket, state, claims))
}

async fn websocket(socket: WebSocket, state: Arc<AppState>, claims: Claims) {
    // by splitting, we can send and receive at the same time
    let (mut sender, mut receiver) = socket.split();

    // create a mpsc channel for passing message
    let (tx, mut rx) = mpsc::channel(CHAN_CAPACITY);
    let client = Client::new(claims.user_id, claims.room_id, tx);

    // this task will receive message from mpsc channel and send to client
    let mut send_task = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(15));
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    if sender.send(msg).await.is_err() {
                        break;
                    }
                }
                _ = interval.tick() => {
                    if sender.send(Message::Ping(Vec::new())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // this task will receive client message and process
    let mut recv_task = {
        let state = state.clone();
        let client = client.clone();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        tracing::debug!("event: {}", text);
                        if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                            if event.process(&state, &client).await.is_err() {
                                break;
                            }
                        }
                    }
                    Message::Close(_) => break,
                    Message::Pong(_) => {}
                    _ => {}
                }
            }
        })
    };

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    // Disconnecting the channels
    let _ = state.hub.disconnect(&client).await;
    tracing::debug!("socket disconnect {}:{}", client.user_id(), client.id());
}
