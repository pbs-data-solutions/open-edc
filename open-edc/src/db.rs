use std::time::Duration;

use anyhow::Result;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use sqlx::{
    pool::PoolConnection,
    postgres::{PgPool, PgPoolOptions},
    Postgres,
};

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
