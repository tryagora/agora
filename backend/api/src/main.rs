use axum::{
    routing::get,
    Router,
};
use std::net::socketaddr;
use tower_http::cors::corslayer;
use tracing_subscriber::{layer::subscriberext, util::subscriberinitext};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::envfilter::try_from_default_env()
                .unwrap_or_else(|_| "agora_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = router()
        .layer(corslayer::permissive());

    let listener = tokio::net::tcplistener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind to port 3000");

    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .expect("server failed");
}

fn router() -> router {
    router::new()
        .route("/health", get(health_check))
}

async fn health_check() -> &'static str {
    "ok"
}
