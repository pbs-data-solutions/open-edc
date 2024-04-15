use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

use crate::config::Config;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum HealthStatus {
    Healthy,
    Unhealthy,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct Health {
    server: HealthStatus,
    db: HealthStatus,
}

pub fn health_routes(pool: PgPool, config: &Config) -> Router<PgPool> {
    let prefix = format!("{}/health", config.api_v1_prefix);
    Router::new().route(&prefix, get(health)).with_state(pool)
}

pub async fn health(State(pool): State<PgPool>) -> Response {
    tracing::debug!("Checking server health");

    let db_status = match sqlx::query!("SELECT 1 as result").fetch_one(&pool).await {
        Ok(_) => {
            tracing::debug!("Server is healthy");
            HealthStatus::Healthy
        }
        Err(_) => {
            tracing::debug!("Server is unhealthy");
            HealthStatus::Unhealthy
        }
    };

    Json(Health {
        server: HealthStatus::Healthy,
        db: db_status,
    })
    .into_response()
}
