use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::app_state::AppState;
use crate::matrix::client::MatrixClient;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/rooms", get(list_joined_rooms))
        .route("/rooms/create", post(create_room))
        .route("/rooms/join", post(join_room))
        .route("/rooms/members", get(get_room_members))
        .route("/rooms/invite", post(invite_user))
}

#[derive(Debug, Deserialize)]
pub struct RoomListQuery {
    pub access_token: String,
}

#[derive(Debug, Serialize)]
pub struct RoomListResponse {
    pub rooms: Vec<RoomInfo>,
}

#[derive(Debug, Serialize)]
pub struct RoomInfo {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub is_space: bool,
    pub member_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub access_token: String,
    pub name: String,
    pub topic: Option<String>,
    pub is_space: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinRoomRequest {
    pub access_token: String,
    pub room_id_or_alias: String,
}

#[derive(Debug, Deserialize)]
pub struct RoomMembersQuery {
    pub access_token: String,
    pub room_id: String,
}

#[derive(Debug, Serialize)]
pub struct RoomMembersResponse {
    pub members: Vec<MemberInfo>,
}

#[derive(Debug, Serialize)]
pub struct MemberInfo {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub access_token: String,
    pub room_id: String,
    pub user_id: String,
}

async fn list_joined_rooms(
    state: State<Arc<AppState>>,
    Query(params): Query<RoomListQuery>,
) -> Result<Json<RoomListResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    match matrix.get_joined_rooms().await {
        Ok(response) => {
            let mut rooms = Vec::new();
            
            for room_id in response.joined_rooms {
                // fetch room state to get name and topic
                let name = if let Ok(state_events) = matrix.get_room_state(room_id.clone()).await {
                    state_events
                        .iter()
                        .find(|e| e.event_type == "m.room.name")
                        .and_then(|e| e.content.get("name"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                } else {
                    None
                };

                let topic = if let Ok(state_events) = matrix.get_room_state(room_id.clone()).await {
                    state_events
                        .iter()
                        .find(|e| e.event_type == "m.room.topic")
                        .and_then(|e| e.content.get("topic"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                } else {
                    None
                };

                // check if it's a space
                let is_space = if let Ok(state_events) = matrix.get_room_state(room_id.clone()).await {
                    state_events
                        .iter()
                        .find(|e| e.event_type == "m.room.create")
                        .and_then(|e| e.content.get("type"))
                        .map(|v| v.as_str() == Some("m.space"))
                        .unwrap_or(false)
                } else {
                    false
                };

                rooms.push(RoomInfo {
                    room_id,
                    name,
                    topic,
                    is_space,
                    member_count: None, // would need to fetch member count
                });
            }

            Ok(Json(RoomListResponse { rooms }))
        }
        Err(e) => {
            tracing::error!("failed to get joined rooms: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

async fn create_room(
    state: State<Arc<AppState>>,
    Json(req): Json<CreateRoomRequest>,
) -> Result<Json<CreateRoomResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    match matrix.create_room(req.name, req.topic, req.is_space.unwrap_or(false)).await {
        Ok(response) => {
            Ok(Json(CreateRoomResponse {
                room_id: response.room_id,
            }))
        }
        Err(e) => {
            tracing::error!("failed to create room: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn join_room(
    state: State<Arc<AppState>>,
    Json(req): Json<JoinRoomRequest>,
) -> Result<Json<CreateRoomResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    match matrix.join_room(req.room_id_or_alias).await {
        Ok(response) => {
            Ok(Json(CreateRoomResponse {
                room_id: response.room_id,
            }))
        }
        Err(e) => {
            tracing::error!("failed to join room: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn get_room_members(
    state: State<Arc<AppState>>,
    Query(params): Query<RoomMembersQuery>,
) -> Result<Json<RoomMembersResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    match matrix.get_room_members(params.room_id).await {
        Ok(response) => {
            let members = response
                .members
                .into_iter()
                .map(|m| MemberInfo {
                    user_id: m.user_id,
                    display_name: m.display_name,
                    avatar_url: m.avatar_url,
                })
                .collect();

            Ok(Json(RoomMembersResponse { members }))
        }
        Err(e) => {
            tracing::error!("failed to get room members: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn invite_user(
    state: State<Arc<AppState>>,
    Json(req): Json<InviteRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    match matrix.invite_user(req.room_id, req.user_id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to invite user: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
