pub mod app_state;
pub mod matrix;
pub mod routes;

use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::app_state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agora_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut state = AppState::new();
    
    // initialize database (optional - continues without db if it fails)
    if let Err(e) = state.init_database().await {
        tracing::warn!("database connection failed: {}. continuing without database.", e);
    }
    
    // initialize redis (optional - continues without redis if it fails)
    if let Err(e) = state.init_redis().await {
        tracing::warn!("redis connection failed: {}. continuing without redis.", e);
    }

    let state = Arc::new(state);

    let app = router()
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind to port 3000");

    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .expect("server failed");
}

fn router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(routes::health::router())
        .merge(routes::auth::router())
        .merge(routes::sync::router())
}
