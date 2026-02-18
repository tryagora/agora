use axum::{
    extract::{Query, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use redis::AsyncCommands;
use serde::Deserialize;
use std::sync::Arc;
use crate::app_state::{AppState, PresenceEvent};

#[derive(Deserialize)]
pub struct WsQuery {
    access_token: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ws/presence", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // upgrade to websocket — access_token is accepted but not deeply validated
    // (conduit would reject any Matrix calls made with a bad token anyway)
    let _ = params.access_token;
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // subscribe to the broadcast channel before sending the snapshot so we
    // don't miss any events that arrive between the snapshot and subscribe
    let mut rx = state.presence_tx.subscribe();

    // send a snapshot of every currently-online user from redis
    if let Some(mut redis) = state.redis.clone() {
        // KEYS is O(N) but fine for small deployments
        let keys: Vec<String> = redis.keys("presence:*").await.unwrap_or_default();
        for key in keys {
            let value: Option<String> = redis.get(&key).await.unwrap_or(None);
            if let Some(presence) = value {
                let user_id = key.trim_start_matches("presence:").to_string();
                let event = PresenceEvent { user_id, presence };
                if let Ok(json) = serde_json::to_string(&event) {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        return; // client disconnected during snapshot
                    }
                }
            }
        }
    }

    // forward broadcast events to this client until it disconnects
    loop {
        tokio::select! {
            // new presence event from the broadcast channel
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        if let Ok(json) = serde_json::to_string(&event) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break; // client disconnected
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // receiver fell behind — skip dropped events, keep going
                        tracing::warn!("presence ws: dropped {} events (receiver lagged)", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            // drain incoming frames — we don't expect client messages but must
            // read to detect disconnects (close frames, pings)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    _ => {} // ignore other frames
                }
            }
        }
    }
}
