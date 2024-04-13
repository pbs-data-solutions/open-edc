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
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api::v1::routes,
    cli::{Cli, Command},
    config::Config,
    db::DbClient,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        api::v1::routes::organization::create_organization,
        api::v1::routes::organization::delete_organization,
        api::v1::routes::organization::get_organization,
        api::v1::routes::organization::get_organizations,
        api::v1::routes::organization::update_organization,
        api::v1::routes::user::create_user,
        api::v1::routes::user::delete_user,
        api::v1::routes::user::get_user,
        api::v1::routes::user::get_users,
        api::v1::routes::user::update_user,
    ),
    components(schemas(
        models::organization::Organization,
        models::organization::OrganizationCreate,
        models::organization::OrganizationUpdate,
        models::messages::GenericMessage,
        models::user::User,
        models::user::UserCreate,
        models::user::UserUpdate,
    )),
    tags(
        (name = "Organization", description = "Organization management"),
        (name = "Users", description = "User managmenet"),
    ),
)]
pub struct ApiDoc;

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
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .merge(routes::health::health_routes(pool.clone(), &config))
        .merge(routes::organization::organization_routes(
            pool.clone(),
            &config,
        ))
        .merge(routes::user::user_routes(pool.clone(), &config))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            organization::{Organization, OrganizationCreate},
            user::{User, UserCreate},
        },
        services::{
            organization_services::create_organization_service, user_services::create_user_service,
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
}
