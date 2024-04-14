mod api;
mod cli;
mod config;
mod db;
mod models;
mod openapi;
mod services;
mod utils;

use std::env;

use anyhow::Result;
use axum::{serve, Router};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use clap::Parser;
use dotenvy::dotenv;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api::v1::routes,
    cli::{Cli, Command},
    config::Config,
    db::DbClient,
    openapi::ApiDoc,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // try_from_default_env reads the RUST_LOG environment if set
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "open_edc=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .init();

    let args = Cli::parse();

    match args.command {
        Command::Start { url, port } => {
            let app = app().await;
            let server_url = url.unwrap_or("0.0.0.0".to_string());
            let server_port = port.unwrap_or(3000);
            let listener = tokio::net::TcpListener::bind(format!("{server_url}:{server_port}"))
                .await
                .unwrap();
            tracing::info!("listening on {}", listener.local_addr().unwrap());
            serve(listener, app).await.unwrap();
        }
    }

    Ok(())
}

async fn app() -> Router {
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

    let db_pool = db_client
        .create_pool(None, None)
        .await
        .expect("Unable to connect to the database");

    match sqlx::query!("SELECT 1 as result").fetch_one(&db_pool).await {
        Ok(_) => {
            tracing::debug!("Successfully connected to Postgres and pinged it");
        }
        Err(_) => {
            tracing::debug!("Error connecting to Postgres server");
        }
    };

    tracing::debug!("Connecting to valkey");
    let valkey_address = env::var("VALKEY_ADDRESS").unwrap_or("127.0.0.1".to_string());
    let valkey_password = env::var("VALKEY_PASSWORD").unwrap_or("valkeypassword".to_string());
    let valkey_port = env::var("VALKEY_PORT")
        .unwrap_or("6379".to_string())
        .parse::<u16>()
        .unwrap_or(6379);
    let manager = RedisConnectionManager::new(format!(
        "redis://:{valkey_password}@{valkey_address}:{valkey_port}"
    ))
    .expect("Error creating valkey manager");
    let valkey_pool = Pool::builder()
        .build(manager)
        .await
        .expect("Error creating valkey pool");

    let valkey_pool_clone = valkey_pool.clone();
    let mut conn = valkey_pool_clone
        .get()
        .await
        .expect("Error getting the valkey pool");
    let result: String = redis::cmd("PING")
        .query_async(&mut *conn)
        .await
        .expect("Error pinging valkey server");
    assert_eq!(result, "PONG");
    tracing::debug!("Successfully connected to valkey and pinged it");

    let config = Config::new(None);

    Router::new()
        .layer(TraceLayer::new_for_http())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .merge(routes::health::health_routes(db_pool.clone(), &config))
        .merge(routes::organization::organization_routes(
            db_pool.clone(),
            &config,
        ))
        .merge(routes::study::study_routes(db_pool.clone(), &config))
        .merge(routes::user::user_routes(db_pool.clone(), &config))
        .with_state(db_pool)
        .with_state(valkey_pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            organization::{Organization, OrganizationCreate},
            study::{Study, StudyCreate},
            user::{User, UserCreate},
        },
        services::{
            organization_services::create_organization_service,
            study_services::create_study_service, user_services::create_user_service,
        },
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

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

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

    #[tokio::test]
    async fn create_study() {
        let app = app().await;
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&pool, &create_org)
            .await
            .unwrap();
        let study_id = Uuid::new_v4().to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/study")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "study_id": study_id,
                            "study_name": "Test Study",
                            "study_description": "Description",
                            "organization_id": organization.id,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Study = serde_json::from_slice(&body).unwrap();

        assert_eq!(body.study_id, study_id);
    }

    #[tokio::test]
    async fn get_study() {
        let app = app().await;
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&pool, &create_org)
            .await
            .unwrap();
        let study_create = StudyCreate {
            study_id: Uuid::new_v4().to_string(),
            study_name: Some("Study Name".to_string()),
            study_description: Some("Description".to_string()),
            organization_id: organization.id,
        };
        let study = create_study_service(&pool, &study_create).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/study/{}", &study.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Study = serde_json::from_slice(&body).unwrap();

        assert_eq!(body.study_id, study_create.study_id);
    }

    #[tokio::test]
    async fn get_study_not_found() {
        let study_id = generate_db_id();
        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/study/{}", &study_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_user() {
        let app = app().await;
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&pool, &create_org)
            .await
            .unwrap();
        let user_name = Uuid::new_v4().to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/user")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "user_name": user_name,
                            "first_name": "Arthur",
                            "last_name": "Dent",
                            "email": "arthur@heartofgold.com",
                            "password": "Somepassword1!",
                            "organization_id": organization.id,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(body.user_name, user_name);
    }

    #[tokio::test]
    async fn get_user() {
        let app = app().await;
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&pool, &create_org)
            .await
            .unwrap();
        let user_create = UserCreate {
            user_name: Uuid::new_v4().to_string(),
            first_name: "Imma".to_string(),
            last_name: "Person".to_string(),
            email: "some@email.com".to_string(),
            password: "Somepassword1!".to_string(),
            organization_id: organization.id,
        };
        let user = create_user_service(&pool, &user_create).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/user/{}", &user.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(body.user_name, user_create.user_name);
    }

    #[tokio::test]
    async fn get_user_not_found() {
        let user_id = generate_db_id();
        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/user/{}", &user_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn add_user_to_study() {
        let app = app().await;
        let db_client = db_client();
        let pool = db_client.create_pool(Some(1), None).await.unwrap();
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&pool, &create_org)
            .await
            .unwrap();
        let user_create = UserCreate {
            user_name: Uuid::new_v4().to_string(),
            first_name: "Imma".to_string(),
            last_name: "Person".to_string(),
            email: "some@email.com".to_string(),
            password: "Somepassword1!".to_string(),
            organization_id: organization.id.clone(),
        };
        let user = create_user_service(&pool, &user_create).await.unwrap();
        let study_create = StudyCreate {
            study_id: Uuid::new_v4().to_string(),
            study_name: Some("Study Name".to_string()),
            study_description: Some("Description".to_string()),
            organization_id: organization.id.clone(),
        };
        let study = create_study_service(&pool, &study_create).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/user/study")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "user_id": user.id,
                            "study_id": study.id,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(body.id, user.id);
        assert!(body.studies.is_some());
        let studies_test = body.studies.unwrap();
        assert_eq!(studies_test.len(), 1);
    }
}
