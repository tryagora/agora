use axum::{
    Router,
    routing::get,
};
use std::sync::Arc;
use crate::app_state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
}

async fn health_check() -> &'static str {
    "ok"
}
