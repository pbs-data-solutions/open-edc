mod api;
mod cli;
mod config;
mod db;
mod models;
mod utils;

use std::env;

use anyhow::Result;
use axum::{serve, Router};
use clap::Parser;
use dotenvy::dotenv;

use crate::{
    api::v1::routes,
    cli::{Cli, Command},
    config::Config,
    db::DbClient,
};

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
    );
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
    use crate::{
        models::organization::{create_organization_service, Organization, OrganizationCreate},
        utils::generate_db_id,
    };
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt; // for `collect`
    use serde_json::{json, Value};
    use tower::ServiceExt; // for `oneshot`
    use uuid::Uuid;

    fn db_client() -> DbClient {
        DbClient::new("127.0.0.1", "postgres", "test_password", &5432, "open_edc")
    }

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
    async fn create_organization() {
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

    #[tokio::test]
    async fn delete_organization() {
        let org_name = Uuid::new_v4().to_string();
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate { name: org_name };
        let new_org = create_organization_service(&pool, &create_org)
            .await
            .unwrap();

        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(&format!("/api/v1/organization/{}", &new_org.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let result = sqlx::query_as!(
            Organization,
            r#"
                SELECT id, name, active, date_added, date_modified
                FROM organizations
                WHERE id = $1
            "#,
            &new_org.id,
        )
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_organization_not_found() {
        let org_id = generate_db_id();
        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(&format!("/api/v1/organization/{}", &org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_organization() {
        let org_name = Uuid::new_v4().to_string();
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate { name: org_name };
        let new_org = create_organization_service(&pool, &create_org)
            .await
            .unwrap();

        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/organization/{}", &new_org.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Organization = serde_json::from_slice(&body).unwrap();

        assert_eq!(body.name, create_org.name);
    }

    #[tokio::test]
    async fn get_organization_not_found() {
        let org_id = generate_db_id();
        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/organization/{}", &org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_organizations() {
        let org_name = Uuid::new_v4().to_string();
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate { name: org_name };
        create_organization_service(&pool, &create_org)
            .await
            .unwrap();

        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/organization")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Vec<Organization> = serde_json::from_slice(&body).unwrap();
        println!("{:?}", body);

        assert!(body.iter().any(|item| item.name == create_org.name));
    }

    #[tokio::test]
    async fn update_organization() {
        let org_name = Uuid::new_v4().to_string();
        let app = app().await;
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate { name: org_name };
        let new_org = create_organization_service(&pool, &create_org)
            .await
            .unwrap();

        let updated_name = Uuid::new_v4().to_string();
        let active = false;
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/api/v1/organization")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(
                            &json!({"id": new_org.id, "name": updated_name, "active": active }),
                        )
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Organization = serde_json::from_slice(&body).unwrap();

        assert_eq!(body.name, updated_name);
        assert_eq!(body.active, active);
    }
}
