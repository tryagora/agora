use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub db_pool: Option<sqlx::PgPool>,
    pub redis: Option<redis::aio::ConnectionManager>,
    pub matrix_client: Arc<RwLock<Option<crate::matrix::client::MatrixClient>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db_pool: None,
            redis: None,
            matrix_client: Arc::new(RwLock::new(None)),
        }
    }
}
