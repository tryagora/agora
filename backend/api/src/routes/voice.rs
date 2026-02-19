use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::app_state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/voice/token", post(get_voice_token))
        .route("/voice/participants", get(get_voice_participants))
        .route("/voice/call", post(send_call_event))
        .route("/voice/vibe", get(get_vibe))
        .route("/voice/vibe", post(set_vibe))
}

#[derive(Debug, Deserialize)]
pub struct VoiceTokenRequest {
    pub access_token: String,
    pub room_id: String,
    pub user_id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VoiceTokenResponse {
    pub token: String,
    pub livekit_url: String,
}

#[derive(Debug, Deserialize)]
pub struct VoiceParticipantsQuery {
    pub room_name: String,
}

#[derive(Debug, Serialize)]
pub struct VoiceParticipantsResponse {
    pub participants: Vec<String>,
}

// livekit jwt claims — matches the livekit server spec exactly
#[derive(Debug, Serialize, Deserialize)]
struct LiveKitClaims {
    // standard jwt fields
    exp: usize,
    iss: String,
    // livekit-specific: room name the token grants access to
    #[serde(rename = "jti")]
    jti: String,
    // livekit video grant
    video: VideoGrant,
    // livekit metadata (display name etc)
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VideoGrant {
    #[serde(rename = "roomJoin")]
    room_join: bool,
    room: String,
    #[serde(rename = "canPublish")]
    can_publish: bool,
    #[serde(rename = "canSubscribe")]
    can_subscribe: bool,
    #[serde(rename = "canPublishData")]
    can_publish_data: bool,
}

async fn get_voice_token(
    _state: State<Arc<AppState>>,
    Json(req): Json<VoiceTokenRequest>,
) -> Result<Json<VoiceTokenResponse>, StatusCode> {
    let api_key = std::env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "devkey".to_string());
    let api_secret = std::env::var("LIVEKIT_API_SECRET")
        .unwrap_or_else(|_| "devsecret_agora_local_development_key_32chars".to_string());
    let livekit_url = std::env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://localhost:7880".to_string());

    // use the matrix room id as the livekit room name (sanitized)
    // strip leading ! and replace : with _ for livekit compatibility
    let room_name = sanitize_room_name(&req.room_id);

    // token valid for 6 hours
    let exp = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 6 * 3600) as usize;

    let claims = LiveKitClaims {
        exp,
        iss: api_key.clone(),
        // jti is the participant identity (their matrix user_id)
        jti: req.user_id.clone(),
        video: VideoGrant {
            room_join: true,
            room: room_name.clone(),
            can_publish: true,
            can_subscribe: true,
            can_publish_data: true,
        },
        name: req.display_name,
    };

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(api_secret.as_bytes());

    match jsonwebtoken::encode(&header, &claims, &key) {
        Ok(token) => Ok(Json(VoiceTokenResponse { token, livekit_url })),
        Err(e) => {
            tracing::error!("failed to generate livekit token: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_voice_participants(
    _state: State<Arc<AppState>>,
    Query(params): Query<VoiceParticipantsQuery>,
) -> Result<Json<VoiceParticipantsResponse>, StatusCode> {
    let api_key = std::env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "devkey".to_string());
    let api_secret = std::env::var("LIVEKIT_API_SECRET")
        .unwrap_or_else(|_| "devsecret_agora_local_development_key_32chars".to_string());
    let livekit_http = std::env::var("LIVEKIT_HTTP_URL")
        .unwrap_or_else(|_| "http://localhost:7880".to_string());

    // generate an admin token to call the livekit rest api
    let admin_token = match make_admin_token(&api_key, &api_secret) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("failed to make admin token: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let room_name = sanitize_room_name(&params.room_name);
    let url = format!("{}/twirp/livekit.RoomService/ListParticipants", livekit_http);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "room": room_name }))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let participants = body["participants"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|p| p["identity"].as_str().map(String::from))
                .collect();
            Ok(Json(VoiceParticipantsResponse { participants }))
        }
        Ok(r) => {
            let status = r.status().as_u16();
            // 404 = room doesn't exist yet (no one joined) — normal, return empty
            // 401 = bad jwt or livekit just restarted — log at debug, not warn
            if status == 404 || status == 401 {
                if status == 401 {
                    tracing::debug!("livekit participants 401 — jwt may be stale or livekit restarted");
                }
                return Ok(Json(VoiceParticipantsResponse { participants: vec![] }));
            }
            tracing::warn!("livekit list participants returned unexpected {}", status);
            Ok(Json(VoiceParticipantsResponse { participants: vec![] }))
        }
        Err(e) => {
            // livekit unreachable — return empty list rather than 500
            tracing::warn!("livekit unreachable for participants: {}", e);
            Ok(Json(VoiceParticipantsResponse { participants: vec![] }))
        }
    }
}

/// generate a short-lived admin jwt for livekit rest api calls.
/// livekit requires: iss = api_key, sub = identity, video grant with roomAdmin/roomList.
/// the `sub` field is the caller identity — livekit rejects tokens without it (401).
fn make_admin_token(api_key: &str, api_secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 60) as usize; // 1 minute is enough for a rest call

    let claims = serde_json::json!({
        "exp": exp,
        "iss": api_key,
        "sub": "agora-server",   // required by livekit — identity of the caller
        "video": {
            "roomList": true,
            "roomAdmin": true
        }
    });

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(api_secret.as_bytes());
    jsonwebtoken::encode(&header, &claims, &key)
}

// ── call signaling ────────────────────────────────────────────────────────────
// calls are signaled via special Matrix messages (msgtype: agora.call)
// the sync loop on each client detects these and triggers the incoming call ui

#[derive(Debug, Deserialize)]
pub struct CallEventRequest {
    pub access_token: String,
    /// the matrix dm room id to send the event into
    pub room_id: String,
    /// "ring" | "accept" | "cancel"
    pub action: String,
    /// unique id for this call session — matches ring/accept/cancel together
    pub call_id: String,
    pub from_user_id: String,
    pub display_name: Option<String>,
}

async fn send_call_event(
    state: State<Arc<AppState>>,
    Json(req): Json<CallEventRequest>,
) -> Result<StatusCode, StatusCode> {
    use crate::matrix::client::MatrixClient;

    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    let content = serde_json::json!({
        "msgtype": "agora.call",
        "body": format!("[call {}]", req.action),
        "call_id": req.call_id,
        "action": req.action,
        "from": req.from_user_id,
        "display_name": req.display_name.unwrap_or_default(),
    });

    match matrix.send_message_content(req.room_id, content).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to send call event: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// ── vibe rooms ────────────────────────────────────────────────────────────────
// vibe is stored as a matrix state event (agora.vibe) on the voice channel room.
// any participant can set it; everyone polling /voice/vibe sees the change.

#[derive(Debug, Deserialize)]
pub struct VibeQuery {
    pub access_token: String,
    pub room_id: String,
}

#[derive(Debug, Serialize)]
pub struct VibeResponse {
    pub vibe: String, // "none" | "rain" | "lofi" | "campfire" | "space"
    pub set_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetVibeRequest {
    pub access_token: String,
    pub room_id: String,
    /// the vibe id to set — must be one of: none, rain, lofi, campfire, space
    pub vibe: String,
    pub user_id: String,
}

async fn get_vibe(
    state: State<Arc<AppState>>,
    Query(params): Query<VibeQuery>,
) -> Result<Json<VibeResponse>, StatusCode> {
    use crate::matrix::client::MatrixClient;

    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    // read the agora.vibe state event from the room
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/state/agora.vibe/",
        state.homeserver_url,
        urlencoding_encode(&params.room_id)
    );

    let resp = matrix.get_raw(&url).await;
    match resp {
        Ok(body) => {
            let vibe = body["vibe"].as_str().unwrap_or("none").to_string();
            let set_by = body["set_by"].as_str().map(String::from);
            Ok(Json(VibeResponse { vibe, set_by }))
        }
        Err(_) => {
            // no vibe set yet (404 from conduit) — return none
            Ok(Json(VibeResponse { vibe: "none".to_string(), set_by: None }))
        }
    }
}

async fn set_vibe(
    state: State<Arc<AppState>>,
    Json(req): Json<SetVibeRequest>,
) -> Result<StatusCode, StatusCode> {
    use crate::matrix::client::MatrixClient;

    // validate vibe value server-side
    let allowed = ["none", "rain", "lofi", "campfire", "space"];
    if !allowed.contains(&req.vibe.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    let content = serde_json::json!({
        "vibe": req.vibe,
        "set_by": req.user_id,
    });

    match matrix.send_state_event(req.room_id, "agora.vibe".to_string(), "".to_string(), content).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("failed to set vibe: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// url-encode a matrix room id for use in a path segment
fn urlencoding_encode(s: &str) -> String {
    s.chars().map(|c| match c {
        '!' => "%21".to_string(),
        ':' => "%3A".to_string(),
        '.' => "%2E".to_string(),
        '#' => "%23".to_string(),
        _ => c.to_string(),
    }).collect()
}

/// sanitize a matrix room id into a livekit-compatible room name
/// matrix room ids look like: !abc123:localhost
/// livekit room names must not contain special chars the jwt can't handle
fn sanitize_room_name(room_id: &str) -> String {
    room_id
        .trim_start_matches('!')
        .replace(':', "_")
        .replace('.', "_")
}
