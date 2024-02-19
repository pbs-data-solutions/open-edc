mod api;
mod cli;
mod config;
mod db;
mod models;
mod services;
mod utils;

use std::env;

use anyhow::Result;
use axum::{serve, Router};
use clap::Parser;
use dotenvy::dotenv;

use crate::api::v1::routes;
use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::db::DbClient;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let args = Cli::parse();

    match args.command {
        Command::Start { url, port } => {
            let app = app().await;
            let server_url = url.unwrap_or("0.0.0.0".to_string());
            let server_port = port.unwrap_or(3000);
            let listener = tokio::net::TcpListener::bind(format!("{server_url}:{server_port}"))
                .await
                .unwrap();

            serve(listener, app).await.unwrap();
        }
    }

    Ok(())
}

async fn app() -> Router {
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
    )
    .await
    .expect("Unable to connect to database");
    let pool = db_client
        .create_pool(None, None)
        .await
        .expect("Unable to connect to the database");

    let config = Config::new(None);

    Router::new()
        .merge(routes::organization::organization_routes(
            pool.clone(),
            &config,
        ))
        .merge(routes::health::health_routes(pool.clone(), &config))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt; // for `collect`
    use serde_json::{json, Value};
    use tower::ServiceExt; // for `oneshot`
    use uuid::Uuid;

    #[tokio::test]
    async fn get_health() {
        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({ "db": "healthy".to_string(), "server": "healthy".to_string() })
        );
    }

    #[tokio::test]
    async fn add_organization() {
        let app = app().await;
        let name = Uuid::new_v4().to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/organization")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "name": name })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
