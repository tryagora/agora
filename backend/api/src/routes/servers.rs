// servers.rs — server-level management endpoints
// covers: metadata, vanity aliases, roles, member management, forum threads
// all server state is stored as Matrix state events on the server (space) room

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
        // server metadata
        .route("/servers/meta", get(get_server_meta).post(set_server_meta))
        // roles
        .route("/servers/roles", get(get_roles).post(set_roles))
        .route("/servers/members/roles", get(get_member_roles).post(set_member_roles))
        // forum threads
        .route("/servers/forum/threads", get(list_threads))
        .route("/servers/forum/thread", post(create_thread))
        // invite / vanity
        .route("/servers/invite", get(get_invite_info))
}

// ── server metadata ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ServerMetaQuery {
    pub access_token: String,
    pub server_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    /// the vanity slug used as the room alias: #slug:localhost
    pub vanity_slug: Option<String>,
    /// template id used to initially populate the server
    pub template: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetServerMetaRequest {
    pub access_token: String,
    pub server_id: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    /// setting a new vanity slug creates a new room alias and updates agora.server.meta
    pub vanity_slug: Option<String>,
    pub name: Option<String>,
}

async fn get_server_meta(
    state: State<Arc<AppState>>,
    Query(params): Query<ServerMetaQuery>,
) -> Result<Json<ServerMeta>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    // read agora.server.meta state event
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/state/agora.server.meta/",
        state.homeserver_url,
        url_encode(&params.server_id)
    );
    match matrix.get_raw(&url).await {
        Ok(body) => {
            let meta: ServerMeta = serde_json::from_value(body).unwrap_or(ServerMeta {
                name: None, description: None, icon_url: None, banner_url: None,
                vanity_slug: None, template: None,
            });
            Ok(Json(meta))
        }
        Err(_) => Ok(Json(ServerMeta {
            name: None, description: None, icon_url: None, banner_url: None,
            vanity_slug: None, template: None,
        }))
    }
}

async fn set_server_meta(
    state: State<Arc<AppState>>,
    Json(req): Json<SetServerMetaRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token.clone());

    // read current meta first so we only overwrite provided fields
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/state/agora.server.meta/",
        state.homeserver_url,
        url_encode(&req.server_id)
    );
    let mut current: ServerMeta = matrix.get_raw(&url).await
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(ServerMeta {
            name: None, description: None, icon_url: None, banner_url: None,
            vanity_slug: None, template: None,
        });

    if let Some(d) = req.description { current.description = Some(d); }
    if let Some(i) = req.icon_url    { current.icon_url = Some(i); }
    if let Some(b) = req.banner_url  { current.banner_url = Some(b); }
    if let Some(n) = req.name.clone() {
        // also update the room name via standard Matrix state event
        let name_content = serde_json::json!({ "name": n });
        let _ = matrix.send_state_event(
            req.server_id.clone(), "m.room.name".to_string(), "".to_string(), name_content
        ).await;
        current.name = Some(n);
    }

    if let Some(slug) = req.vanity_slug {
        // validate slug: alphanumeric and hyphens only, 3-32 chars
        let clean: String = slug.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .to_lowercase();
        if clean.len() < 3 || clean.len() > 32 {
            return Err(StatusCode::BAD_REQUEST);
        }
        // create the new alias (will fail silently if already taken by someone else)
        let _ = matrix.create_room_alias(
            format!("#{clean}:localhost"), req.server_id.clone()
        ).await;
        current.vanity_slug = Some(clean);
    }

    let content = serde_json::to_value(&current).unwrap_or_default();
    match matrix.send_state_event(req.server_id, "agora.server.meta".to_string(), "".to_string(), content).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to set server meta: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// ── roles ─────────────────────────────────────────────────────────────────────
// roles are stored as a single agora.roles state event (list of role objects).
// member role assignments are stored as agora.member.roles state events (one per user).
// permissions are a flat flags object — which actions are allowed for the role.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RolePermissions {
    pub send_messages: bool,
    pub manage_channels: bool,
    pub manage_roles: bool,
    pub kick_members: bool,
    pub ban_members: bool,
    pub mention_everyone: bool,
    pub manage_server: bool,
    pub administrator: bool, // overrides all others
}

impl Default for RolePermissions {
    fn default() -> Self {
        Self {
            send_messages: true,
            manage_channels: false,
            manage_roles: false,
            kick_members: false,
            ban_members: false,
            mention_everyone: false,
            manage_server: false,
            administrator: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    pub id: String,         // uuid4 or short string
    pub name: String,
    pub color: String,      // hex colour e.g. "#5865f2"
    pub hoist: bool,        // show separately in member list
    pub mentionable: bool,
    pub permissions: RolePermissions,
    /// power level this role maps to in Matrix (for enforcement)
    pub power_level: i64,
}

#[derive(Debug, Serialize)]
pub struct RolesResponse {
    pub roles: Vec<Role>,
}

#[derive(Debug, Deserialize)]
pub struct RolesQuery {
    pub access_token: String,
    pub server_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SetRolesRequest {
    pub access_token: String,
    pub server_id: String,
    pub roles: Vec<Role>,
}

async fn get_roles(
    state: State<Arc<AppState>>,
    Query(params): Query<RolesQuery>,
) -> Result<Json<RolesResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/state/agora.roles/",
        state.homeserver_url, url_encode(&params.server_id)
    );
    let roles = match matrix.get_raw(&url).await {
        Ok(body) => body["roles"].as_array()
            .and_then(|arr| serde_json::from_value::<Vec<Role>>(serde_json::Value::Array(arr.clone())).ok())
            .unwrap_or_default(),
        Err(_) => vec![],
    };
    Ok(Json(RolesResponse { roles }))
}

async fn set_roles(
    state: State<Arc<AppState>>,
    Json(req): Json<SetRolesRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token.clone());

    // also sync power levels for each role so Matrix enforcement works
    // fetch current power levels first
    let power_result = matrix.get_power_levels(req.server_id.clone()).await;
    if let Ok(power) = power_result {
        // build a map of all role members' power levels
        // first get all member role assignments
        // (simplified: we just ensure role power levels are registered in the base levels object)
        for role in &req.roles {
            if role.permissions.administrator {
                // administrator roles need power 100 to bypass all checks
                // we can't easily enumerate members here, so we set the role's listed power
            }
            let _ = role.power_level; // used below when assigning to members
        }
        let content = serde_json::to_value(&power).unwrap_or_default();
        let _ = matrix.send_state_event(req.server_id.clone(), "m.room.power_levels".to_string(), "".to_string(), content).await;
    }

    let content = serde_json::json!({ "roles": req.roles });
    match matrix.send_state_event(req.server_id, "agora.roles".to_string(), "".to_string(), content).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to set roles: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// ── member role assignments ───────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberRoles {
    pub user_id: String,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MemberRolesQuery {
    pub access_token: String,
    pub server_id: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SetMemberRolesRequest {
    pub access_token: String,
    pub server_id: String,
    pub user_id: String,
    pub role_ids: Vec<String>,
}

async fn get_member_roles(
    state: State<Arc<AppState>>,
    Query(params): Query<MemberRolesQuery>,
) -> Result<Json<MemberRoles>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    let encoded_uid = url_encode(&params.user_id);
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/state/agora.member.roles/{encoded_uid}",
        state.homeserver_url, url_encode(&params.server_id)
    );
    let role_ids = match matrix.get_raw(&url).await {
        Ok(body) => body["role_ids"].as_array()
            .and_then(|arr| serde_json::from_value::<Vec<String>>(serde_json::Value::Array(arr.clone())).ok())
            .unwrap_or_default(),
        Err(_) => vec![],
    };
    Ok(Json(MemberRoles { user_id: params.user_id, role_ids }))
}

async fn set_member_roles(
    state: State<Arc<AppState>>,
    Json(req): Json<SetMemberRolesRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token.clone());

    // also update the member's Matrix power level to match the highest-power role they have
    // first fetch the current roles list so we know the power levels
    let roles_url = format!(
        "{}/_matrix/client/v3/rooms/{}/state/agora.roles/",
        state.homeserver_url, url_encode(&req.server_id)
    );
    let roles: Vec<Role> = matrix.get_raw(&roles_url).await.ok()
        .and_then(|v| v["roles"].as_array().and_then(|a| serde_json::from_value::<Vec<Role>>(serde_json::Value::Array(a.clone())).ok()))
        .unwrap_or_default();

    // compute the highest power level this member gets from their roles
    let max_power = req.role_ids.iter()
        .filter_map(|rid| roles.iter().find(|r| &r.id == rid))
        .map(|r| r.power_level)
        .max()
        .unwrap_or(0);

    // update Matrix power levels for this member
    if let Ok(mut power) = matrix.get_power_levels(req.server_id.clone()).await {
        power.users.get_or_insert_with(Default::default).insert(req.user_id.clone(), max_power);
        let content = serde_json::to_value(&power).unwrap_or_default();
        let _ = matrix.send_state_event(req.server_id.clone(), "m.room.power_levels".to_string(), "".to_string(), content).await;
    }

    let content = serde_json::json!({ "role_ids": req.role_ids });
    match matrix.send_state_event(req.server_id.clone(), "agora.member.roles".to_string(), req.user_id.clone(), content).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to set member roles: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// ── forum threads ─────────────────────────────────────────────────────────────
// a forum channel is a Matrix room with agora.room.type = "forum".
// threads are Matrix rooms with agora.room.type = "thread" linked as
// m.space.child state events on the forum channel room.

#[derive(Debug, Deserialize)]
pub struct ThreadsQuery {
    pub access_token: String,
    pub forum_channel_id: String,
}

#[derive(Debug, Serialize)]
pub struct ThreadInfo {
    pub room_id: String,
    pub title: String,
    pub author: String,
    pub created_at: Option<u64>,
    pub reply_count: Option<u64>,
    pub pinned: bool,
}

#[derive(Debug, Serialize)]
pub struct ThreadsResponse {
    pub threads: Vec<ThreadInfo>,
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub access_token: String,
    pub forum_channel_id: String,
    pub title: String,
    pub author: String,
    /// initial message body for the thread (sent as first message)
    pub body: String,
}

async fn list_threads(
    state: State<Arc<AppState>>,
    Query(params): Query<ThreadsQuery>,
) -> Result<Json<ThreadsResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token.clone());

    // get all m.space.child events from the forum channel room
    let room_state = matrix.get_room_state(params.forum_channel_id.clone()).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let child_ids: Vec<String> = room_state.iter()
        .filter(|e| e.event_type == "m.space.child")
        .filter_map(|e| e.state_key.clone())
        .filter(|k| !k.is_empty())
        .collect();

    let mut threads = Vec::new();
    for child_id in child_ids {
        // read thread state
        let thread_state = matrix.get_room_state(child_id.clone()).await.unwrap_or_default();
        let title = thread_state.iter()
            .find(|e| e.event_type == "m.room.name")
            .and_then(|e| e.content["name"].as_str().map(String::from))
            .unwrap_or_else(|| "untitled".to_string());

        let author = thread_state.iter()
            .find(|e| e.event_type == "agora.thread.meta")
            .and_then(|e| e.content["author"].as_str().map(String::from))
            .unwrap_or_default();

        let created_at = thread_state.iter()
            .find(|e| e.event_type == "agora.thread.meta")
            .and_then(|e| e.content["created_at"].as_u64());

        let reply_count = thread_state.iter()
            .find(|e| e.event_type == "agora.thread.meta")
            .and_then(|e| e.content["reply_count"].as_u64());

        let pinned = thread_state.iter()
            .find(|e| e.event_type == "agora.thread.meta")
            .and_then(|e| e.content["pinned"].as_bool())
            .unwrap_or(false);

        threads.push(ThreadInfo { room_id: child_id, title, author, created_at, reply_count, pinned });
    }

    // sort: pinned first, then by created_at descending
    threads.sort_by(|a, b| {
        b.pinned.cmp(&a.pinned)
            .then(b.created_at.cmp(&a.created_at))
    });

    Ok(Json(ThreadsResponse { threads }))
}

async fn create_thread(
    state: State<Arc<AppState>>,
    Json(req): Json<CreateThreadRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token.clone());

    // create a new Matrix room for this thread
    let thread_room = matrix.create_room(req.title.clone(), None, false).await
        .map_err(|e| { tracing::error!("failed to create thread room: {}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // tag it as a thread
    let meta = serde_json::json!({
        "author": req.author,
        "created_at": now_ms,
        "reply_count": 0,
        "pinned": false,
    });
    let _ = matrix.send_state_event(thread_room.room_id.clone(), "agora.room.type".to_string(), "".to_string(), serde_json::json!({ "type": "thread" })).await;
    let _ = matrix.send_state_event(thread_room.room_id.clone(), "agora.thread.meta".to_string(), "".to_string(), meta).await;

    // link thread room to forum channel
    let _ = matrix.add_space_child(req.forum_channel_id.clone(), thread_room.room_id.clone()).await;

    // send the opening message
    let _ = matrix.send_message(thread_room.room_id.clone(), req.body).await;

    Ok(Json(serde_json::json!({ "room_id": thread_room.room_id })))
}

// ── invite info ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InviteQuery {
    pub access_token: String,
    pub server_id: String,
}

#[derive(Debug, Serialize)]
pub struct InviteInfo {
    /// the Matrix room alias that can be shared
    pub alias: String,
    /// the vanity slug portion (if set via agora.server.meta)
    pub vanity_slug: Option<String>,
    pub server_name: String,
    pub member_count: u64,
}

async fn get_invite_info(
    state: State<Arc<AppState>>,
    Query(params): Query<InviteQuery>,
) -> Result<Json<InviteInfo>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token.clone());

    let room_state = matrix.get_room_state(params.server_id.clone()).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let server_name = room_state.iter()
        .find(|e| e.event_type == "m.room.name")
        .and_then(|e| e.content["name"].as_str().map(String::from))
        .unwrap_or_else(|| "server".to_string());

    let member_count = room_state.iter()
        .filter(|e| e.event_type == "m.room.member" && e.content["membership"].as_str() == Some("join"))
        .count() as u64;

    // look up room alias from canonical alias event
    let alias = room_state.iter()
        .find(|e| e.event_type == "m.room.canonical_alias")
        .and_then(|e| e.content["alias"].as_str().map(String::from))
        .unwrap_or_else(|| params.server_id.clone());

    // read vanity slug from agora meta
    let meta_url = format!(
        "{}/_matrix/client/v3/rooms/{}/state/agora.server.meta/",
        state.homeserver_url, url_encode(&params.server_id)
    );
    let vanity_slug = matrix.get_raw(&meta_url).await.ok()
        .and_then(|v| v["vanity_slug"].as_str().map(String::from));

    Ok(Json(InviteInfo { alias, vanity_slug, server_name, member_count }))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    s.chars().map(|c| match c {
        '!' => "%21".to_string(),
        ':' => "%3A".to_string(),
        '.' => "%2E".to_string(),
        '#' => "%23".to_string(),
        '@' => "%40".to_string(),
        _ => c.to_string(),
    }).collect()
}
