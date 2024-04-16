use std::env;

use anyhow::{bail, Result};
use axum::extract::FromRef;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use sqlx::postgres::PgPool;

use crate::db::DbClient;

#[derive(Clone)]
pub struct DbState {
    pub pool: PgPool,
}

impl FromRef<AppState> for DbState {
    fn from_ref(app_state: &AppState) -> DbState {
        app_state.db_state.clone()
    }
}

impl DbState {
    pub async fn create_state() -> Result<Self> {
        tracing::debug!("Connecting to postgres");
        let address = env::var("DATABASE_ADDRESS").unwrap_or("127.0.0.1".to_string());
        let user = env::var("DATASE_USER").unwrap_or("postgres".to_string());
        let user_password = env::var("DATASE_USER_PASSWORD").unwrap_or("test_password".to_string());
        let port = env::var("DATABASE_PORT")
            .unwrap_or("5432".to_string())
            .parse::<u16>()
            .unwrap_or(5432);
        let db_client = DbClient::new(&address, &user, &user_password, &port, "open_edc");

        let pool = match db_client.create_pool(None, None).await {
            Ok(p) => p,
            Err(e) => bail!("Unable to connect to the database: {}", e.to_string()),
        };

        match sqlx::query!("SELECT 1 as result").fetch_one(&pool).await {
            Ok(_) => tracing::debug!("Successfully connected to Postgres and pinged it"),
            Err(_) => bail!("Error connecting to Postgres server"),
        };

        let state = Self { pool: pool.clone() };

        Ok(state)
    }
}

#[derive(Clone)]
pub struct ValkeyState {
    pub pool: Pool<RedisConnectionManager>,
}

impl FromRef<AppState> for ValkeyState {
    fn from_ref(app_state: &AppState) -> ValkeyState {
        app_state.valkey_state.clone()
    }
}

impl ValkeyState {
    pub async fn create_state() -> Result<Self> {
        tracing::debug!("Connecting to valkey");
        let address = env::var("VALKEY_ADDRESS").unwrap_or("127.0.0.1".to_string());
        let password = env::var("VALKEY_PASSWORD").unwrap_or("valkeypassword".to_string());
        let port = env::var("VALKEY_PORT")
            .unwrap_or("6379".to_string())
            .parse::<u16>()
            .unwrap_or(6379);
        let manager =
            match RedisConnectionManager::new(format!("redis://:{password}@{address}:{port}")) {
                Ok(m) => m,
                Err(e) => bail!("Error creating valkey manager: {}", e.to_string()),
            };
        let pool = match Pool::builder().build(manager).await {
            Ok(p) => p,
            Err(e) => bail!("Error creating valkey pool: {}", e.to_string()),
        };

        let pool_clone = pool.clone();
        let mut conn = match pool_clone.get().await {
            Ok(c) => c,
            Err(e) => bail!("Error getting the valkey pool: {}", e.to_string()),
        };
        let result: String = match redis::cmd("PING").query_async(&mut *conn).await {
            Ok(r) => r,
            Err(e) => bail!("Error pinging valkey server: {}", e.to_string()),
        };

        if result != "PONG" {
            bail!("Unable to ping valkey server");
        }

        let state = Self { pool: pool.clone() };
        tracing::debug!("Successfully connected to valkey and pinged it");

        Ok(state)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db_state: DbState,
    pub valkey_state: ValkeyState,
}

impl AppState {
    pub async fn create_state() -> Result<Self> {
        tracing::debug!("Creating db_state");
        let db_state = match DbState::create_state().await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Error creating db_state: {}", e.to_string());
                panic!("Unable to connect to database");
            }
        };
        tracing::debug!("Successfully created db_state");

        tracing::debug!("Creating valkey_state");
        let valkey_state = match ValkeyState::create_state().await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Error creating valkey_state: {}", e.to_string());
                panic!("Unable to connect to valkey");
            }
        };
        tracing::debug!("Successfully created valkey_state");

        Ok(Self {
            db_state,
            valkey_state,
        })
    }
}
