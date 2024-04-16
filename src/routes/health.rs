use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{config::Config, state::AppState};

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
    valkey: HealthStatus,
}

pub fn health_routes(state: Arc<AppState>, config: &Config) -> Router<Arc<AppState>> {
    let prefix = format!("{}/health", config.api_prefix);
    Router::new().route(&prefix, get(health)).with_state(state)
}

pub async fn health(State(state): State<Arc<AppState>>) -> Response {
    tracing::debug!("Checking db health");
    let db_pool = state.db_state.pool.clone();

    let db_status = match sqlx::query!("SELECT 1 as result").fetch_one(&db_pool).await {
        Ok(_) => {
            tracing::debug!("db is healthy");
            HealthStatus::Healthy
        }
        Err(_) => {
            tracing::debug!("db is unhealthy");
            HealthStatus::Unhealthy
        }
    };

    tracing::debug!("Checking valkey health");
    let valkey_status: HealthStatus;

    match state.valkey_state.pool.get().await {
        Ok(mut conn) => {
            let result: String = redis::cmd("PING")
                .query_async(&mut *conn)
                .await
                .unwrap_or("unhealthy".to_string());
            if result == "PONG" {
                tracing::debug!("valkey is healthy");
                valkey_status = HealthStatus::Healthy;
            } else {
                tracing::debug!("valkey is unhealthy");
                valkey_status = HealthStatus::Unhealthy;
            }
        }
        Err(_) => {
            tracing::debug!("valkey is unhealthy");
            valkey_status = HealthStatus::Unhealthy;
        }
    }

    Json(Health {
        server: HealthStatus::Healthy,
        db: db_status,
        valkey: valkey_status,
    })
    .into_response()
}
