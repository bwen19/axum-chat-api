use super::user_socket::UserSocket;
use crate::{extractor::SocketGuard, ws::event::ServerEvent, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::{
    sync::mpsc,
    time::{self, Duration},
};

// ========================// Websocket Handler //======================== //

/// Handler of the websocket router
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    skg: SocketGuard,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket(socket, state, skg))
}

/// Actual websocket statemachine
async fn websocket(socket: WebSocket, state: Arc<AppState>, skg: SocketGuard) {
    // create a mpsc channel for passing message
    let capacity = state.config.user_channel_capacity;
    let (tx, mut rx) = mpsc::channel(capacity);

    let uss = UserSocket::new(skg, tx);
    if let Err(e) = state.channel.close(&uss).await {
        tracing::error!("{}", e.to_string());
        return;
    }

    // by splitting, we can send and receive at the same time
    let (mut sender, mut receiver) = socket.split();

    // this task will receive message from mpsc channel and send to client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // the task will be terminated if the closing signal is sent
            if let Message::Close(_) = msg {
                let _ = sender.send(msg).await;
                break;
            };

            if sender.send(msg).await.is_err() {
                break;
            }
        }
        tracing::debug!("Close socket from send task");
    });

    let mut heart_task = {
        let tx = uss.tx.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(15));
            loop {
                interval.tick().await;
                if let Ok(msg) = ServerEvent::Ping(123).to_msg() {
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
            tracing::debug!("Close socket from heart task");
        })
    };

    // state used in receive task
    let recv_state = state.clone();
    let recv_uss = uss.clone();

    // this task will receive client message and process
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(event) = serde_json::from_str(&text) {
                        if !recv_uss.handle_event(&recv_state, event).await {
                            break;
                        }
                    } else {
                        tracing::debug!("Receive text message from client: {}", text);
                    }
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
    });

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
            heart_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
            heart_task.abort();
        }
        _ = (&mut heart_task) => {
            send_task.abort();
            recv_task.abort();
        }
    }

    // Disconnecting the channels
    state.channel.disconnect(&uss);
    tracing::debug!("Disconnect WebSocket {} {}", uss.user_id, uss.socket_id);
}
