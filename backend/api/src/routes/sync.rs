use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::app_state::AppState;
use crate::matrix::client::MatrixClient;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sync", get(sync))
}

#[derive(Debug, Deserialize)]
pub struct SyncQuery {
    pub access_token: String,
    pub since: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub next_batch: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub room_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: Option<i64>,
}

async fn sync(
    state: State<Arc<AppState>>,
    Query(params): Query<SyncQuery>,
) -> Result<Json<SyncResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);
    
    match matrix.sync(params.since).await {
        Ok(response) => {
            let mut messages = Vec::new();
            
            if let Some(rooms) = response.rooms {
                if let Some(join) = rooms.join {
                    for (room_id, room) in join {
                        if let Some(timeline) = room.timeline {
                            for event in timeline.events {
                                if event.event_type == "m.room.message" {
                                    let content = event.content.get("body")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    
                                    messages.push(Message {
                                        room_id: room_id.clone(),
                                        sender: event.sender,
                                        content,
                                        timestamp: event.origin_server_ts,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(Json(SyncResponse {
                next_batch: response.next_batch,
                messages,
            }))
        }
        Err(e) => {
            tracing::error!("sync failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
