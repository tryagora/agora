use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::app_state::AppState;
use crate::matrix::client::MatrixClient;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub access_token: String,
    pub home_server: Option<String>,
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub access_token: String,
    pub home_server: Option<String>,
    pub device_id: Option<String>,
}

async fn register(
    state: State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let matrix = MatrixClient::new(state.homeserver_url.clone());
    
    match matrix.register(req.username, req.password).await {
        Ok(response) => {
            // extract home_server from user_id if not provided (e.g., "@user:localhost" -> "localhost")
            let home_server = response.home_server.or_else(|| {
                response.user_id.split(':').nth(1).map(String::from)
            });
            
            Ok(Json(RegisterResponse {
                user_id: response.user_id,
                access_token: response.access_token,
                home_server,
                device_id: response.device_id,
            }))
        }
        Err(e) => {
            tracing::error!("registration failed: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn login(
    state: State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let matrix = MatrixClient::new(state.homeserver_url.clone());
    
    // ensure username is in full user_id format (@user:server)
    let user = if req.username.starts_with('@') {
        req.username
    } else {
        format!("@{}:localhost", req.username)
    };
    
    match matrix.login(user, req.password).await {
        Ok(response) => {
            // extract home_server from user_id if not provided
            let home_server = response.home_server.or_else(|| {
                response.user_id.split(':').nth(1).map(String::from)
            });
            
            Ok(Json(LoginResponse {
                user_id: response.user_id,
                access_token: response.access_token,
                home_server,
                device_id: response.device_id,
            }))
        }
        Err(e) => {
            tracing::error!("login failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
