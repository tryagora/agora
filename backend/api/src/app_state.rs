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

    pub async fn init_database(&mut self) -> Result<(), sqlx::Error> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://agora:agora_dev_password@localhost:5432/agora".to_string());
        
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;
        
        // run migrations
        sqlx::migrate!("./migrations").run(&pool).await.ok();
        
        self.db_pool = Some(pool);
        tracing::info!("database connected");
        Ok(())
    }

    pub async fn init_redis(&mut self) -> Result<(), redis::RedisError> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_multiplexed_tokio_connection().await?;
        
        self.redis = Some(conn);
        tracing::info!("redis connected");
        Ok(())
    }
}
