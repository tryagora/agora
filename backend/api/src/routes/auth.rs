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
    pub home_server: String,
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
    pub home_server: String,
}

async fn register(
    _state: State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    // todo: implement actual matrix registration via conduit
    // for now, return mock response
    Ok(Json(RegisterResponse {
        user_id: format!("@{}:localhost", req.username),
        access_token: "mock_token_".to_string() + &uuid::Uuid::new_v4().to_string(),
        home_server: "localhost".to_string(),
    }))
}

async fn login(
    _state: State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // todo: implement actual matrix login via conduit
    Ok(Json(LoginResponse {
        user_id: format!("@{}:localhost", req.username),
        access_token: "mock_token_".to_string() + &uuid::Uuid::new_v4().to_string(),
        home_server: "localhost".to_string(),
    }))
}
