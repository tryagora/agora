use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub db_pool: Option<sqlx::PgPool>,
    pub redis: Option<redis::aio::MultiplexedConnection>,
    pub matrix_client: Arc<RwLock<Option<crate::matrix::client::MatrixClient>>>,
    pub homeserver_url: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db_pool: None,
            redis: None,
            matrix_client: Arc::new(RwLock::new(None)),
            homeserver_url: std::env::var("CONDUIT_URL")
                .unwrap_or_else(|_| "http://localhost:8448".to_string()),
        }
    }
}
