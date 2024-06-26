mod cli;
mod config;
mod db;
mod models;
mod openapi;
mod routes;
mod services;
mod state;
mod utils;

use std::sync::Arc;

use anyhow::Result;
use axum::{serve, Router};
use clap::Parser;
use dotenvy::dotenv;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    cli::{Cli, Command},
    config::Config,
    openapi::ApiDoc,
    state::AppState,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_env("LOG_LEVEL")
                .unwrap_or_else(|_| "open_edc=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .init();

    let args = Cli::parse();

    match args.command {
        Command::Start {} => {
            let config = Config::new();
            let app = app(&config).await;
            let server_url = &config.server_url;
            let server_port = &config.port;
            let listener = tokio::net::TcpListener::bind(format!("{server_url}:{server_port}"))
                .await
                .unwrap();
            tracing::info!("listening on {}", listener.local_addr().unwrap());
            serve(listener, app).await.unwrap();
        }
    }

    Ok(())
}

async fn app(config: &Config) -> Router {
    let app_state = match AppState::create_state(config).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Error creating state: {}", e.to_string());
            panic!("Error creating state, cannot start server");
        }
    };
    let state = Arc::new(app_state);

    Router::new()
        .layer(TraceLayer::new_for_http())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .merge(routes::health::health_routes(state.clone(), config))
        .merge(routes::organization::organization_routes(
            state.clone(),
            config,
        ))
        .merge(routes::study::study_routes(state.clone(), config))
        .merge(routes::user::user_routes(state.clone(), config))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use bb8::Pool;
    use bb8_redis::RedisConnectionManager;
    use http_body_util::BodyExt; // for `collect`
    use serde_json::{json, Value};
    use tower::ServiceExt; // for `oneshot`
    use uuid::Uuid;

    use crate::{
        db::DbClient,
        models::{
            organization::{Organization, OrganizationCreate},
            study::{Study, StudyCreate, StudyInDb},
            user::{AccessLevel, User, UserCreate, UserInDb},
        },
        services::{
            organization_services::create_organization_service,
            study_services::create_study_service, user_services::create_user_service,
        },
        utils::generate_db_id,
    };

    fn db_client() -> DbClient {
        DbClient::new("127.0.0.1", "postgres", "test_password", &5432, "open_edc")
    }

    async fn valkey_pool() -> Pool<RedisConnectionManager> {
        let valkey_address = "127.0.0.1".to_string();
        let valkey_password = "valkeypassword".to_string();
        let valkey_port = 6379;
        let manager = RedisConnectionManager::new(format!(
            "redis://:{valkey_password}@{valkey_address}:{valkey_port}"
        ))
        .expect("Error creating valkey manager");

        Pool::builder()
            .build(manager)
            .await
            .expect("Error creating valkey pool")
    }

    fn config() -> Config {
        dotenv().ok();
        Config::new()
    }

    #[tokio::test]
    async fn get_health() {
        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
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
            json!({ "db": "healthy".to_string(), "server": "healthy".to_string(), "valkey": "healthy".to_string() })
        );
    }

    #[tokio::test]
    async fn create_organization() {
        let app = app(&config()).await;
        let name = Uuid::new_v4().to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/organization")
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
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate { name: org_name };
        let new_org = create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();

        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(&format!("/api/organization/{}", &new_org.id))
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
        .fetch_optional(&db_pool)
        .await
        .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_organization_not_found() {
        let org_id = generate_db_id();
        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(&format!("/api/organization/{}", &org_id))
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
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate { name: org_name };
        let new_org = create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();

        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/organization/{}", &new_org.id))
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
        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/organization/{}", &org_id))
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
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate { name: org_name };
        create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();

        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/organization")
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
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate { name: org_name };
        let new_org = create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();

        let updated_name = Uuid::new_v4().to_string();
        let active = false;
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/api/organization")
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
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();
        let study_id = Uuid::new_v4().to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/study")
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
    async fn delete_study() {
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();
        let study_create = StudyCreate {
            study_id: Uuid::new_v4().to_string(),
            study_name: Some("Study Name".to_string()),
            study_description: Some("Description".to_string()),
            organization_id: organization.id,
        };
        let study = create_study_service(&db_pool, &valkey_pool, &study_create)
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(&format!("/api/study/{}", &study.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let result = sqlx::query_as!(
            StudyInDb,
            r#"
                SELECT
                    id,
                    study_id,
                    study_name,
                    study_description,
                    organization_id,
                    date_added,
                    date_modified
                FROM studies
                WHERE id = $1
            "#,
            &study.id,
        )
        .fetch_optional(&db_pool)
        .await
        .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_study() {
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();
        let study_create = StudyCreate {
            study_id: Uuid::new_v4().to_string(),
            study_name: Some("Study Name".to_string()),
            study_description: Some("Description".to_string()),
            organization_id: organization.id,
        };
        let study = create_study_service(&db_pool, &valkey_pool, &study_create)
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/study/{}", &study.id))
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
        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/study/{}", &study_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_user() {
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&db_pool, &valkey_pool, &create_org)
            .await
            .unwrap();
        let user_name = Uuid::new_v4().to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/user")
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
    async fn delete_user() {
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&db_pool, &valkey_pool, &create_org)
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
        let user = create_user_service(&db_pool, &valkey_pool, &user_create)
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(&format!("/api/user/{}", &user.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let result = sqlx::query_as!(
            UserInDb,
            r#"
                SELECT
                    id,
                    user_name,
                    first_name,
                    last_name,
                    email,
                    hashed_password,
                    organization_id,
                    active,
                    access_level AS "access_level: AccessLevel",
                    date_added,
                    date_modified
                FROM users
                WHERE id = $1
            "#,
            &user.id,
        )
        .fetch_optional(&db_pool)
        .await
        .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_user() {
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&db_pool, &valkey_pool, &create_org)
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
        let user = create_user_service(&db_pool, &valkey_pool, &user_create)
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/user/{}", &user.id))
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
        let app = app(&config()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/user/{}", &user_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn add_user_to_study() {
        let app = app(&config()).await;
        let db_client = db_client();
        let db_pool = db_client.create_pool(Some(1), None).await.unwrap();
        let valkey_pool = valkey_pool().await;
        let create_org = OrganizationCreate {
            name: Uuid::new_v4().to_string(),
        };
        let organization = create_organization_service(&db_pool, &valkey_pool, &create_org)
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
        let user = create_user_service(&db_pool, &valkey_pool, &user_create)
            .await
            .unwrap();
        let study_create = StudyCreate {
            study_id: Uuid::new_v4().to_string(),
            study_name: Some("Study Name".to_string()),
            study_description: Some("Description".to_string()),
            organization_id: organization.id.clone(),
        };
        let study = create_study_service(&db_pool, &valkey_pool, &study_create)
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/user/study")
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
