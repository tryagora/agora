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
        .route("/rooms/leave", post(leave_room))
        .route("/rooms/delete", post(delete_room))
        .route("/rooms/members", get(get_room_members))
        .route("/rooms/invite", post(invite_user))
        .route("/rooms/send", post(send_message))
        .route("/rooms/children", get(get_space_children))
        .route("/rooms/remove_child", post(remove_space_child))
        .route("/rooms/state", get(get_room_state))
        .route("/rooms/category/create", post(create_category))
        .route("/rooms/permissions", get(get_permissions).post(set_permissions))
        .route("/rooms/raid", post(send_raid))
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
    /// "text" or "voice" — defaults to "text" if the state event is absent
    pub channel_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub access_token: String,
    pub name: String,
    pub topic: Option<String>,
    pub is_space: Option<bool>,
    pub parent_space_id: Option<String>,
    /// "text" (default) or "voice"
    pub channel_type: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub access_token: String,
    pub room_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct RoomStateQuery {
    pub access_token: String,
    pub room_id: String,
}

#[derive(Debug, Serialize)]
pub struct RoomStateResponse {
    pub events: Vec<RoomStateEvent>,
}

#[derive(Debug, Serialize)]
pub struct RoomStateEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub sender: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub event_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SpaceChildrenQuery {
    pub access_token: String,
    pub space_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoveChildRequest {
    pub access_token: String,
    pub space_id: String,
    pub child_room_id: String,
}

#[derive(Debug, Serialize)]
pub struct SpaceChildrenResponse {
    pub children: Vec<RoomInfo>,
}

#[derive(Debug, Deserialize)]
pub struct LeaveRoomRequest {
    pub access_token: String,
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRoomRequest {
    pub access_token: String,
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub access_token: String,
    pub name: String,
    pub parent_space_id: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCategoryResponse {
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PermissionsQuery {
    pub access_token: String,
    pub room_id: String,
}

#[derive(Debug, Serialize)]
pub struct PermissionsResponse {
    pub users: std::collections::HashMap<String, i64>,
    pub users_default: i64,
}

#[derive(Debug, Deserialize)]
pub struct SetPermissionsRequest {
    pub access_token: String,
    pub room_id: String,
    pub user_id: String,
    pub power_level: i64,
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
                // fetch state once — if this fails (403, user already left) skip the room entirely
                // this prevents ghost rooms from appearing in the list after a partial leave
                let state_events = match matrix.get_room_state(room_id.clone()).await {
                    Ok(events) => events,
                    Err(e) => {
                        tracing::debug!("skipping room {} — cannot read state (likely already left): {}", room_id, e);
                        continue;
                    }
                };

                let name = state_events
                    .iter()
                    .find(|e| e.event_type == "m.room.name")
                    .and_then(|e| e.content.get("name"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let topic = state_events
                    .iter()
                    .find(|e| e.event_type == "m.room.topic")
                    .and_then(|e| e.content.get("topic"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let is_space = state_events
                    .iter()
                    .find(|e| e.event_type == "m.room.create")
                    .and_then(|e| e.content.get("type"))
                    .map(|v| v.as_str() == Some("m.space"))
                    .unwrap_or(false);

                let channel_type = state_events
                    .iter()
                    .find(|e| e.event_type == "agora.room.type")
                    .and_then(|e| e.content.get("type"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "text".to_string());

                rooms.push(RoomInfo {
                    room_id,
                    name,
                    topic,
                    is_space,
                    member_count: None,
                    channel_type: Some(channel_type),
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
    matrix.access_token = Some(req.access_token.clone());

    let parent_space_id = req.parent_space_id.clone();
    let room_name = req.name.clone();
    let is_space = req.is_space.unwrap_or(false);
    let channel_type = req.channel_type.clone().unwrap_or_else(|| "text".to_string());

    match matrix.create_room(req.name.clone(), req.topic.clone(), is_space).await {
        Ok(response) => {
            let room_id = response.room_id.clone();

            // store the channel type as a Matrix state event so all clients can read it
            // store for all non-space channels (text, voice, forum) so the frontend
            // can reliably distinguish them without falling back to defaults
            if !is_space {
                let content = serde_json::json!({ "type": channel_type });
                if let Err(e) = matrix.send_state_event(
                    room_id.clone(),
                    "agora.room.type".to_string(),
                    "".to_string(),
                    content,
                ).await {
                    tracing::warn!("failed to set channel type state event: {}", e);
                }
            }

            // create a room alias so users can join by name
            // normalize the name: lowercase, replace spaces with dashes, remove special chars
            let alias_localpart = room_name
                .to_lowercase()
                .replace(' ', "-")
                .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "");
            if !alias_localpart.is_empty() {
                let alias = format!("#{alias_localpart}:localhost");
                if let Err(e) = matrix.create_room_alias(alias, room_id.clone()).await {
                    tracing::warn!("failed to create room alias: {}", e);
                    // don't fail — room was created, just the alias failed
                }
            }

            // if this room has a parent space, add it as a space child
            if let Some(space_id) = parent_space_id.clone() {
                if let Err(e) = matrix.add_space_child(space_id, room_id.clone()).await {
                    tracing::warn!("failed to add space child relationship: {}", e);
                    // don't fail the whole request — room was created, just the hierarchy link failed
                }
            }

            // note: we do NOT auto-create a "general" channel here.
            // the wizard (CreateServerWizard.svelte) creates all channels based on the
            // chosen template, so auto-creating one here would produce duplicates.

            Ok(Json(CreateRoomResponse {
                room_id,
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

    // normalize the input — matrix requires ! for room ids or # for aliases
    let room_id_or_alias = {
        let input = req.room_id_or_alias.trim().to_string();
        if input.starts_with('!') || input.starts_with('#') {
            // already has a sigil — if no server part, append :localhost
            if input.contains(':') {
                input
            } else {
                format!("{}:localhost", input)
            }
        } else {
            // bare name — treat as alias
            format!("#{}:localhost", input)
        }
    };
    tracing::info!("joining room: {}", room_id_or_alias);

    match matrix.join_room(room_id_or_alias).await {
        Ok(response) => {
            let room_id = response.room_id.clone();

            // if the joined room is a space, also join all child channels
            // so members can immediately read and write in the channels
            if let Ok(state_events) = matrix.get_room_state(room_id.clone()).await {
                // check if it's a space
                let is_space = state_events.iter().any(|e| {
                    e.event_type == "m.room.create"
                        && e.content.get("type").and_then(|v| v.as_str()) == Some("m.space")
                });

                if is_space {
                    // get all child room ids from m.space.child events
                    let child_ids: Vec<String> = state_events
                        .iter()
                        .filter(|e| e.event_type == "m.space.child")
                        .filter_map(|e| e.state_key.clone())
                        .filter(|k| !k.is_empty())
                        .collect();

                    for child_id in child_ids {
                        if let Err(e) = matrix.join_room(child_id.clone()).await {
                            tracing::warn!("failed to auto-join child channel {}: {}", child_id, e);
                        } else {
                            tracing::info!("auto-joined child channel: {}", child_id);
                        }
                    }
                }
            }

            Ok(Json(CreateRoomResponse { room_id }))
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
            // filter for actual joined members, extract info from state events
            let members = response
                .members
                .into_iter()
                .filter(|m| {
                    m.event_type == "m.room.member"
                        && m.content.membership.as_deref() == Some("join")
                })
                .map(|m| MemberInfo {
                    user_id: m.state_key,
                    display_name: m.content.display_name,
                    avatar_url: m.content.avatar_url,
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

async fn send_message(
    state: State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    match matrix.send_message(req.room_id, req.content).await {
        Ok(result) => {
            let event_id = result
                .get("event_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(Json(SendMessageResponse { event_id }))
        }
        Err(e) => {
            tracing::error!("failed to send message: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn get_space_children(
    state: State<Arc<AppState>>,
    Query(params): Query<SpaceChildrenQuery>,
) -> Result<Json<SpaceChildrenResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token.clone());

    // get space state events to find m.space.child entries
    let state_events = matrix.get_room_state(params.space_id.clone()).await
        .map_err(|e| {
            tracing::error!("failed to get space state: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    let child_room_ids: Vec<String> = state_events
        .iter()
        .filter(|e| e.event_type == "m.space.child")
        .filter_map(|e| e.state_key.clone())
        .filter(|key| !key.is_empty())
        .collect();

    let mut children = Vec::new();

    for room_id in child_room_ids {
        // single state fetch per child — extract all fields in one pass
        let (name, topic, is_space, channel_type) =
            if let Ok(room_state) = matrix.get_room_state(room_id.clone()).await {
                let name = room_state
                    .iter()
                    .find(|e| e.event_type == "m.room.name")
                    .and_then(|e| e.content.get("name"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let topic = room_state
                    .iter()
                    .find(|e| e.event_type == "m.room.topic")
                    .and_then(|e| e.content.get("topic"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let is_space = room_state
                    .iter()
                    .find(|e| e.event_type == "m.room.create")
                    .and_then(|e| e.content.get("type"))
                    .and_then(|v| v.as_str())
                    .map(|t| t == "m.space")
                    .unwrap_or(false);

                let channel_type = room_state
                    .iter()
                    .find(|e| e.event_type == "agora.room.type")
                    .and_then(|e| e.content.get("type"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "text".to_string());

                (name, topic, is_space, channel_type)
            } else {
                (None, None, false, "text".to_string())
            };

        children.push(RoomInfo {
            room_id,
            name,
            topic,
            is_space,
            member_count: None,
            channel_type: Some(channel_type),
        });
    }

    Ok(Json(SpaceChildrenResponse { children }))
}

async fn get_room_state(
    state: State<Arc<AppState>>,
    Query(params): Query<RoomStateQuery>,
) -> Result<Json<RoomStateResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    match matrix.get_room_state(params.room_id).await {
        Ok(state_events) => {
            let events = state_events
                .into_iter()
                .map(|e| RoomStateEvent {
                    event_type: e.event_type,
                    sender: e.sender,
                    content: e.content,
                })
                .collect();
            Ok(Json(RoomStateResponse { events }))
        }
        Err(e) => {
            tracing::error!("failed to get room state: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn leave_room(
    state: State<Arc<AppState>>,
    Json(req): Json<LeaveRoomRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    // if this is a space, leave all child rooms first so they don't linger in joined_rooms
    if let Ok(state_events) = matrix.get_room_state(req.room_id.clone()).await {
        let is_space = state_events.iter().any(|e| {
            e.event_type == "m.room.create"
                && e.content.get("type").and_then(|v| v.as_str()) == Some("m.space")
        });

        if is_space {
            let child_ids: Vec<String> = state_events
                .iter()
                .filter(|e| e.event_type == "m.space.child")
                .filter_map(|e| e.state_key.clone())
                .filter(|k| !k.is_empty())
                .collect();

            for child_id in child_ids {
                // ignore errors — child may already be left or never joined
                if let Err(e) = matrix.leave_room(child_id.clone()).await {
                    tracing::warn!("could not leave child room {} (may already be left): {}", child_id, e);
                }
                // forget so it doesn't appear in joined_rooms or DM list
                if let Err(e) = matrix.forget_room(child_id.clone()).await {
                    tracing::warn!("could not forget child room {}: {}", child_id, e);
                }
            }
        }
    }

    // leave the room itself — treat "not a member" (M_FORBIDDEN) as success
    match matrix.leave_room(req.room_id.clone()).await {
        Ok(_) => {
            // forget so it's fully removed from the joined list
            if let Err(e) = matrix.forget_room(req.room_id).await {
                tracing::warn!("could not forget room after leaving: {}", e);
            }
            Ok(StatusCode::OK)
        }
        Err(e) => {
            let err_str = e.to_string();
            // conduit returns M_FORBIDDEN when the user isn't a member — treat as success
            if err_str.contains("M_FORBIDDEN") || err_str.contains("not a member") || err_str.contains("not invited or joined") {
                tracing::info!("user already not a member of room, treating leave as success");
                let _ = matrix.forget_room(req.room_id).await;
                Ok(StatusCode::OK)
            } else {
                tracing::error!("failed to leave room: {}", e);
                Err(StatusCode::BAD_REQUEST)
            }
        }
    }
}

async fn delete_room(
    state: State<Arc<AppState>>,
    Json(req): Json<DeleteRoomRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    // for channels/categories, we should remove them from parent space first
    // then leave and forget
    // Note: in matrix, you can't truly "delete" a room, only leave it
    // for a proper delete, we'd need to kick all members and purge from db
    
    match matrix.leave_room(req.room_id.clone()).await {
        Ok(_) => {
            // try to forget, but don't fail if it doesn't work
            if let Err(e) = matrix.forget_room(req.room_id).await {
                tracing::warn!("failed to forget room after leaving: {}", e);
            }
            Ok(StatusCode::OK)
        }
        Err(e) => {
            tracing::error!("failed to leave room: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn create_category(
    state: State<Arc<AppState>>,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<Json<CreateCategoryResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    match matrix.create_category(req.name, req.parent_space_id).await {
        Ok(response) => Ok(Json(CreateCategoryResponse {
            room_id: response.room_id,
        })),
        Err(e) => {
            tracing::error!("failed to create category: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn get_permissions(
    state: State<Arc<AppState>>,
    Query(params): Query<PermissionsQuery>,
) -> Result<Json<PermissionsResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    match matrix.get_power_levels(params.room_id).await {
        Ok(power_levels) => Ok(Json(PermissionsResponse {
            users: power_levels.users.unwrap_or_default(),
            users_default: power_levels.users_default.unwrap_or(0),
        })),
        Err(e) => {
            tracing::error!("failed to get permissions: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn set_permissions(
    state: State<Arc<AppState>>,
    Json(req): Json<SetPermissionsRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token.clone());

    // first get current power levels
    let current = match matrix.get_power_levels(req.room_id.clone()).await {
        Ok(pl) => pl,
        Err(e) => {
            tracing::error!("failed to get current power levels: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // update the specific user's power level
    let mut users = current.users.unwrap_or_default();
    users.insert(req.user_id, req.power_level);

    let power_levels_req = crate::matrix::client::PowerLevelsRequest {
        users,
        users_default: current.users_default,
        events: current.events,
        events_default: current.events_default,
        state_default: current.state_default,
        ban: current.ban,
        kick: current.kick,
        redact: current.redact,
        invite: current.invite,
    };

    match matrix.set_power_levels(req.room_id, power_levels_req).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to set permissions: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn remove_space_child(
    state: State<Arc<AppState>>,
    Json(req): Json<RemoveChildRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    match matrix.remove_space_child(req.space_id, req.child_room_id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to remove space child: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// ── raid alert ────────────────────────────────────────────────────────────────
// a raid message (agora.raid) sent into the server's channel triggers a
// full-screen alert overlay on every member's client via the sync loop.

#[derive(Debug, Deserialize)]
pub struct RaidRequest {
    pub access_token: String,
    /// the channel room to broadcast the raid into
    pub room_id: String,
    pub raider_id: String,
    pub raider_name: String,
    /// optional custom message shown on the raid overlay (e.g. "let's go!!!")
    pub message: Option<String>,
    /// countdown seconds before the raid begins (default 5)
    pub countdown: Option<u32>,
}

async fn send_raid(
    state: State<Arc<AppState>>,
    Json(req): Json<RaidRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    let countdown = req.countdown.unwrap_or(5).min(30); // cap at 30 seconds
    let message = req.message.unwrap_or_else(|| "RAID!".to_string());

    let content = serde_json::json!({
        "msgtype": "agora.raid",
        "body": format!("[raid] {} is raiding!", req.raider_name),
        "raider_id": req.raider_id,
        "raider_name": req.raider_name,
        "message": message,
        "countdown": countdown,
    });

    match matrix.send_message_content(req.room_id, content).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to send raid event: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
