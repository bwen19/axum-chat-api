//! Handlers for websocket

use super::{extractor::CookieGuard, AppState};
use crate::api::event::ClientEvent;
use crate::{conn::Client, util::token::Claims};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::{response::IntoResponse, routing::get, Router};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

// ========================// WebSocket Router //======================== //

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(ws_handler))
}

// ========================// Websocket Handler //======================== //

/// Handler of the websocket router
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    CookieGuard(claims): CookieGuard,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket(socket, state, claims))
}

async fn websocket(socket: WebSocket, state: Arc<AppState>, claims: Claims) {
    // by splitting, we can send and receive at the same time
    let (mut sender, mut receiver) = socket.split();

    // create a mpsc channel for passing message
    let (tx, mut rx) = mpsc::channel(100);
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
                    let _ = sender.send(Message::Ping(Vec::default())).await;
                }
            }
        }
        tracing::debug!("Close socket from send task");
    });

    // this task will receive client message and process
    let mut recv_task = {
        let state = state.clone();
        let client = client.clone();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                            if event.process(&state, &client).await {
                                break;
                            }
                        } else {
                            tracing::debug!("Receive text message from client: {}", text);
                        }
                        tracing::debug!("Receive close message from client");
                    }
                    Message::Close(_) => {
                        tracing::debug!("Receive close message from client");
                        break;
                    }
                    _ => {
                        tracing::debug!("Receive other message from client");
                    }
                }
            }
            tracing::debug!("Close socket from recv task");
        })
    };

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    // Disconnecting the channels
    let _ = state.hub.disconnect(&client).await;
    tracing::debug!("Disconnect WebSocket {} {}", client.user_id(), client.id());
}
