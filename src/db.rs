use std::{env, time::Duration};

use anyhow::{bail, Result};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use sqlx::{
    pool::PoolConnection,
    postgres::{PgPool, PgPoolOptions},
    Postgres,
};

use crate::state::{DbState, ValkeyState};

#[derive(Clone, Debug)]
pub struct DbClient {
    pub uri: String,
}

impl DbClient {
    pub fn new(url: &str, user_name: &str, password: &str, port: &u16, db_name: &str) -> Self {
        let uri = format!("postgresql://{user_name}:{password}@{url}:{port}/{db_name}");

        DbClient { uri }
    }

    pub async fn create_pool(
        &self,
        max_connections: Option<u32>,
        acquire_timeout: Option<Duration>,
    ) -> Result<PgPool> {
        let connections = if let Some(m) = max_connections { m } else { 10 };
        let timeout = if let Some(t) = acquire_timeout {
            t
        } else {
            Duration::from_secs(5)
        };
        let pool = PgPoolOptions::new()
            .max_connections(connections)
            .acquire_timeout(timeout)
            .connect(&self.uri)
            .await?;

        Ok(pool)
    }
}

#[allow(dead_code)]
struct DbManager(PoolConnection<Postgres>);

#[async_trait]
impl<S> FromRequestParts<S> for DbManager
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);
        let conn = pool.acquire().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

pub async fn create_db_state() -> Result<DbState> {
    tracing::debug!("Connecting to postgres");
    let database_address = env::var("DATABASE_ADDRESS").unwrap_or("127.0.0.1".to_string());
    let database_user = env::var("DATASE_USER").unwrap_or("postgres".to_string());
    let database_user_password =
        env::var("DATASE_USER_PASSWORD").unwrap_or("test_password".to_string());
    let database_port = env::var("DATABASE_PORT")
        .unwrap_or("5432".to_string())
        .parse::<u16>()
        .unwrap_or(5432);
    let db_client = DbClient::new(
        &database_address,
        &database_user,
        &database_user_password,
        &database_port,
        "open_edc",
    );

    let db_pool = match db_client.create_pool(None, None).await {
        Ok(p) => p,
        Err(e) => bail!("Unable to connect to the database: {}", e.to_string()),
    };

    match sqlx::query!("SELECT 1 as result").fetch_one(&db_pool).await {
        Ok(_) => tracing::debug!("Successfully connected to Postgres and pinged it"),
        Err(_) => bail!("Error connecting to Postgres server"),
    };

    let db_state = DbState {
        pool: db_pool.clone(),
    };

    Ok(db_state)
}

pub async fn create_valkey_state() -> Result<ValkeyState> {
    tracing::debug!("Connecting to valkey");
    let valkey_address = env::var("VALKEY_ADDRESS").unwrap_or("127.0.0.1".to_string());
    let valkey_password = env::var("VALKEY_PASSWORD").unwrap_or("valkeypassword".to_string());
    let valkey_port = env::var("VALKEY_PORT")
        .unwrap_or("6379".to_string())
        .parse::<u16>()
        .unwrap_or(6379);
    let manager = match RedisConnectionManager::new(format!(
        "redis://:{valkey_password}@{valkey_address}:{valkey_port}"
    )) {
        Ok(m) => m,
        Err(e) => bail!("Error creating valkey manager: {}", e.to_string()),
    };
    let valkey_pool = match Pool::builder().build(manager).await {
        Ok(p) => p,
        Err(e) => bail!("Error creating valkey pool: {}", e.to_string()),
    };

    let valkey_pool_clone = valkey_pool.clone();
    let mut conn = match valkey_pool_clone.get().await {
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

    let valkey_state = ValkeyState {
        pool: valkey_pool.clone(),
    };
    tracing::debug!("Successfully connected to valkey and pinged it");

    Ok(valkey_state)
}
