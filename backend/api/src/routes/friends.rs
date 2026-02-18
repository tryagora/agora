use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sqlx::Row;
use crate::app_state::AppState;
use crate::matrix::client::MatrixClient;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/friends", get(list_friends))
        .route("/friends/add", post(add_friend))
        .route("/friends/accept", post(accept_friend))
        .route("/friends/reject", post(reject_friend))
        .route("/friends/remove", delete(remove_friend))
        .route("/friends/dm", post(get_or_create_dm))
}

// ── request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FriendsQuery {
    pub access_token: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct FriendActionRequest {
    pub access_token: String,
    /// the caller's own matrix user_id
    pub user_id: String,
    /// the other party's matrix user_id
    pub friend_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DmRequest {
    pub access_token: String,
    pub user_id: String,
    pub friend_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FriendEntry {
    pub user_id: String,
    /// "pending_sent" | "pending_received" | "accepted"
    pub status: String,
    pub dm_room_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FriendsListResponse {
    pub friends: Vec<FriendEntry>,
}

#[derive(Debug, Serialize)]
pub struct DmResponse {
    pub room_id: String,
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// require a db pool or return 503
macro_rules! require_db {
    ($state:expr) => {
        match $state.db_pool.as_ref() {
            Some(pool) => pool,
            None => {
                tracing::error!("friends endpoints require a database connection");
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    };
}

// ── handlers ──────────────────────────────────────────────────────────────────

/// list all friends (accepted + pending) for the calling user
async fn list_friends(
    state: State<Arc<AppState>>,
    Query(params): Query<FriendsQuery>,
) -> Result<Json<FriendsListResponse>, StatusCode> {
    let pool = require_db!(state);

    let rows = sqlx::query(
        r#"
        SELECT requester_id, addressee_id, status, dm_room_id
        FROM friends
        WHERE (requester_id = $1 OR addressee_id = $1)
          AND status != 'blocked'
        ORDER BY updated_at DESC
        "#,
    )
    .bind(&params.user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to query friends: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let friends = rows
        .into_iter()
        .map(|row| {
            let requester_id: String = row.get("requester_id");
            let addressee_id: String = row.get("addressee_id");
            let status: String = row.get("status");
            let dm_room_id: Option<String> = row.get("dm_room_id");

            let other = if requester_id == params.user_id {
                addressee_id.clone()
            } else {
                requester_id.clone()
            };

            let status_label = if status == "accepted" {
                "accepted".to_string()
            } else if requester_id == params.user_id {
                "pending_sent".to_string()
            } else {
                "pending_received".to_string()
            };

            FriendEntry {
                user_id: other,
                status: status_label,
                dm_room_id,
            }
        })
        .collect();

    Ok(Json(FriendsListResponse { friends }))
}

/// send a friend request
async fn add_friend(
    state: State<Arc<AppState>>,
    Json(req): Json<FriendActionRequest>,
) -> Result<StatusCode, StatusCode> {
    let pool = require_db!(state);

    if req.user_id == req.friend_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // check for existing relationship in either direction
    let existing = sqlx::query(
        r#"
        SELECT status FROM friends
        WHERE (requester_id = $1 AND addressee_id = $2)
           OR (requester_id = $2 AND addressee_id = $1)
        "#,
    )
    .bind(&req.user_id)
    .bind(&req.friend_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("db error checking existing friend: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(row) = existing {
        let status: String = row.get("status");
        if status == "accepted" {
            return Ok(StatusCode::OK);
        }
        // if they already sent us a request, auto-accept
        if status == "pending" {
            sqlx::query(
                r#"
                UPDATE friends SET status = 'accepted', updated_at = NOW()
                WHERE requester_id = $1 AND addressee_id = $2
                "#,
            )
            .bind(&req.friend_id)
            .bind(&req.user_id)
            .execute(pool)
            .await
            .map_err(|e| {
                tracing::error!("failed to auto-accept friend request: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            return Ok(StatusCode::OK);
        }
    }

    sqlx::query(
        r#"
        INSERT INTO friends (requester_id, addressee_id, status)
        VALUES ($1, $2, 'pending')
        ON CONFLICT (requester_id, addressee_id) DO NOTHING
        "#,
    )
    .bind(&req.user_id)
    .bind(&req.friend_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to insert friend request: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::OK)
}

/// accept an incoming friend request
async fn accept_friend(
    state: State<Arc<AppState>>,
    Json(req): Json<FriendActionRequest>,
) -> Result<StatusCode, StatusCode> {
    let pool = require_db!(state);

    let result = sqlx::query(
        r#"
        UPDATE friends SET status = 'accepted', updated_at = NOW()
        WHERE requester_id = $1 AND addressee_id = $2 AND status = 'pending'
        "#,
    )
    .bind(&req.friend_id)
    .bind(&req.user_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to accept friend request: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::OK)
}

/// reject / decline an incoming friend request
async fn reject_friend(
    state: State<Arc<AppState>>,
    Json(req): Json<FriendActionRequest>,
) -> Result<StatusCode, StatusCode> {
    let pool = require_db!(state);

    sqlx::query(
        r#"
        DELETE FROM friends
        WHERE requester_id = $1 AND addressee_id = $2 AND status = 'pending'
        "#,
    )
    .bind(&req.friend_id)
    .bind(&req.user_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to reject friend request: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::OK)
}

/// remove an accepted friend
async fn remove_friend(
    state: State<Arc<AppState>>,
    Json(req): Json<FriendActionRequest>,
) -> Result<StatusCode, StatusCode> {
    let pool = require_db!(state);

    sqlx::query(
        r#"
        DELETE FROM friends
        WHERE (requester_id = $1 AND addressee_id = $2)
           OR (requester_id = $2 AND addressee_id = $1)
        "#,
    )
    .bind(&req.user_id)
    .bind(&req.friend_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to remove friend: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::OK)
}

/// get the existing DM room for this friendship, or create one and cache it.
/// always ensures the calling user is joined (handles the invite→join transition).
async fn get_or_create_dm(
    state: State<Arc<AppState>>,
    Json(req): Json<DmRequest>,
) -> Result<Json<DmResponse>, StatusCode> {
    let pool = require_db!(state);

    let mut matrix = MatrixClient::new(state.homeserver_url.clone());
    matrix.access_token = Some(req.access_token.clone());

    // look up cached dm_room_id
    let row = sqlx::query(
        r#"
        SELECT dm_room_id FROM friends
        WHERE (requester_id = $1 AND addressee_id = $2)
           OR (requester_id = $2 AND addressee_id = $1)
        "#,
    )
    .bind(&req.user_id)
    .bind(&req.friend_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to look up friend row: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(row) = &row {
        let dm_room_id: Option<String> = row.get("dm_room_id");
        if let Some(room_id) = dm_room_id {
            // ensure the caller is joined — join is idempotent for already-joined members
            // and accepts the pending invite for the invitee
            if let Err(e) = matrix.join_room(room_id.clone()).await {
                tracing::warn!("could not join cached dm room {} (may already be joined): {}", room_id, e);
            }
            return Ok(Json(DmResponse { room_id }));
        }
    }

    // no cached room — create one via matrix.
    // use the short username as the room name so DM list shows a readable label.
    let friend_short = req.friend_id
        .trim_start_matches('@')
        .split(':')
        .next()
        .unwrap_or(&req.friend_id)
        .to_string();

    let create_response = matrix
        .create_dm_room(req.friend_id.clone(), friend_short)
        .await
        .map_err(|e| {
            tracing::error!("failed to create dm room: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    let room_id = create_response.room_id.clone();

    // cache the room id in the friendship row
    sqlx::query(
        r#"
        UPDATE friends SET dm_room_id = $1, updated_at = NOW()
        WHERE (requester_id = $2 AND addressee_id = $3)
           OR (requester_id = $3 AND addressee_id = $2)
        "#,
    )
    .bind(&room_id)
    .bind(&req.user_id)
    .bind(&req.friend_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::warn!("failed to cache dm_room_id: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DmResponse { room_id }))
}
