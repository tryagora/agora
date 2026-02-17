use axum::{
    routing::get,
    router::Router,
};

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health_check))
}

async fn health_check() -> &'static str {
    "ok"
}
