use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Router,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::app_state::{AppState, PresenceEvent};
use crate::matrix::client::MatrixClient;

// how many seconds before a presence key expires automatically.
// if a client crashes without logging out it will go offline after this time.
const PRESENCE_TTL_SECS: u64 = 300; // 5 minutes

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // presence
        .route("/presence/set", post(set_presence))
        .route("/presence/get", get(get_presence))
        // profile
        .route("/profile/get", get(get_profile))
        .route("/profile/set", put(set_profile))
}

// ── types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SetPresenceRequest {
    pub access_token: String,
    pub user_id: String,
    /// "online" | "offline" | "unavailable"
    pub presence: String,
    pub status_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetPresenceQuery {
    pub access_token: String,
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct PresenceResponse {
    pub presence: String,
    pub last_active_ago: Option<i64>,
    pub status_msg: Option<String>,
    pub currently_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GetProfileQuery {
    pub access_token: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SetProfileRequest {
    pub access_token: String,
    pub user_id: String,
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub user_id: String,
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
}

// ── handlers ──────────────────────────────────────────────────────────────────

/// set the calling user's presence state — stored in redis with a TTL so
/// clients that crash without logging out eventually go offline automatically.
async fn set_presence(
    state: State<Arc<AppState>>,
    Json(req): Json<SetPresenceRequest>,
) -> StatusCode {
    let Some(mut redis) = state.redis.clone() else {
        tracing::warn!("set_presence: redis unavailable");
        return StatusCode::SERVICE_UNAVAILABLE;
    };

    // key format: presence:{user_id}
    // value: "online" | "offline" | "unavailable"
    let key = format!("presence:{}", req.user_id);
    let value = req.presence.as_str();

    let result: redis::RedisResult<()> = if value == "offline" {
        // delete immediately so the key doesn't linger
        redis.del(&key).await
    } else {
        // set with TTL so a crash/disconnect eventually expires
        redis.set_ex(&key, value, PRESENCE_TTL_SECS).await
    };

    if let Err(e) = result {
        tracing::warn!("redis set_presence error: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // broadcast the change to all connected websocket clients instantly
    let event = PresenceEvent {
        user_id: req.user_id.clone(),
        presence: req.presence.clone(),
    };
    // send() only errors if there are no receivers — that's fine, just ignore
    let _ = state.presence_tx.send(event);

    StatusCode::OK
}

/// fetch any user's presence state from redis
async fn get_presence(
    state: State<Arc<AppState>>,
    Query(params): Query<GetPresenceQuery>,
) -> Json<PresenceResponse> {
    let Some(mut redis) = state.redis.clone() else {
        tracing::warn!("get_presence: redis unavailable");
        return Json(PresenceResponse {
            presence: "offline".to_string(),
            last_active_ago: None,
            status_msg: None,
            currently_active: Some(false),
        });
    };

    let key = format!("presence:{}", params.user_id);
    let value: Option<String> = redis.get(&key).await.unwrap_or(None);

    let presence = value.unwrap_or_else(|| "offline".to_string());
    let currently_active = presence == "online";

    Json(PresenceResponse {
        presence,
        last_active_ago: None,
        status_msg: None,
        currently_active: Some(currently_active),
    })
}

/// fetch a user's profile (displayname + avatar)
async fn get_profile(
    state: State<Arc<AppState>>,
    Query(params): Query<GetProfileQuery>,
) -> Result<Json<ProfileResponse>, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(params.access_token);

    match matrix.get_profile(params.user_id.clone()).await {
        Ok(p) => Ok(Json(ProfileResponse {
            user_id: params.user_id,
            displayname: p.displayname,
            avatar_url: p.avatar_url,
        })),
        Err(e) => {
            tracing::warn!("failed to get profile for {}: {}", params.user_id, e);
            // return minimal profile rather than an error
            Ok(Json(ProfileResponse {
                user_id: params.user_id,
                displayname: None,
                avatar_url: None,
            }))
        }
    }
}

/// update the calling user's own profile
async fn set_profile(
    state: State<Arc<AppState>>,
    Json(req): Json<SetProfileRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token);

    if let Some(name) = req.displayname {
        matrix
            .set_displayname(req.user_id.clone(), name)
            .await
            .map_err(|e| {
                tracing::warn!("failed to set displayname: {}", e);
                StatusCode::BAD_REQUEST
            })?;
    }

    Ok(StatusCode::OK)
}
