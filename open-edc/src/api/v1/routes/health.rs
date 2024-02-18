use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

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

pub fn health_routes(pool: PgPool) -> Router<PgPool> {
    let prefix = "/health";
    Router::new().route(prefix, get(health)).with_state(pool)
}

pub async fn health(State(pool): State<PgPool>) -> Response {
    let db_status = match sqlx::query!("SELECT 1 as result").fetch_one(&pool).await {
        Ok(_) => HealthStatus::Healthy,
        Err(_) => HealthStatus::Unhealthy,
    };

    Json(Health {
        server: HealthStatus::Healthy,
        db: db_status,
    })
    .into_response()
}
