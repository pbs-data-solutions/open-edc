use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use sqlx::postgres::PgPool;

use crate::{
    config::Config,
    models::{
        messages::GenericMessage,
        organization::{OrganizationCreate, OrganizationUpdate},
    },
    services::organization_services::{
        create_organization_service, delete_organization_service, get_organization_service,
        get_organizations_service, update_organization_service,
    },
};

pub fn organization_routes(pool: PgPool, config: &Config) -> Router<PgPool> {
    let prefix = format!("{}/organization", config.api_v1_prefix);
    Router::new()
        .route(&prefix, post(create_organization))
        .with_state(pool.clone())
        .route(&format!("{prefix}/:id"), delete(delete_organization))
        .with_state(pool.clone())
        .route(&format!("{prefix}/:id"), get(get_organization))
        .with_state(pool.clone())
        .route(&prefix, get(get_organizations))
        .with_state(pool.clone())
        // TODO: I want to make this a patch but need to figure out how to diferentiate between
        // default None and user set None in serde.
        .route(&prefix, put(update_organization))
        .with_state(pool.clone())
}

/// Add a new organization
#[utoipa::path(
    post,
    path = (format!("{}/organization", Config::new(None).api_v1_prefix)),
    request_body = OrganizationCreate,
    tag = "Organization",
    responses(
        (status = 200, description = "Organization added successfully", body = OrganizationCreate),
        (status = 400, description = "Organization already exists", body = GenericMessage)
    )
)]
pub async fn create_organization(
    State(pool): State<PgPool>,
    Json(new_organization): Json<OrganizationCreate>,
) -> Response {
    match create_organization_service(&pool, &new_organization).await {
        Ok(o) => (StatusCode::OK, Json(o)).into_response(),
        Err(e) => {
            if e.to_string().contains("violates unique constraint") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!(
                            "An organization with the name {} already exists",
                            &new_organization.name
                        ),
                    }),
                )
                    .into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenericMessage {
                        detail: "Error adding organization".to_string(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

/// Delete an organization by its database id
#[utoipa::path(
    delete,
    path = (format!("{}/organization/{{id}}", Config::new(None).api_v1_prefix)),
    params(
        ("id" = String, Path, description = "Organization database id")
    ),
    tag = "Organization",
    responses(
        (status = 204, description = "Organization successfully deleted"),
        (status = 404, description = "Organization not found", body = GenericMessage),
    )
)]
pub async fn delete_organization(State(pool): State<PgPool>, Path(id): Path<String>) -> Response {
    match delete_organization_service(&pool, &id).await {
        Ok(o) => (StatusCode::NO_CONTENT, Json(o)).into_response(),
        Err(e) => {
            if e.to_string().contains("No organization with the id") {
                (
                    StatusCode::NOT_FOUND,
                    Json(GenericMessage {
                        detail: e.to_string(),
                    }),
                )
                    .into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenericMessage {
                        detail: "Error deleting organization".to_string(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

/// Get an organization by its database id
#[utoipa::path(
    get,
    path = (format!("{}/organization/{{id}}", Config::new(None).api_v1_prefix)),
    params(
        ("id" = String, Path, description = "Organization database id")
    ),
    tag = "Organization",
    responses(
        (status = 200, description = "Organization information", body = Organization),
        (status = 404, description = "Organization not found", body = GenericMessage),
    )
)]
pub async fn get_organization(State(pool): State<PgPool>, Path(id): Path<String>) -> Response {
    match get_organization_service(&pool, &id).await {
        Ok(organization) => {
            if let Some(o) = organization {
                (StatusCode::OK, Json(o)).into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(GenericMessage {
                        detail: format!("No organization with the id {id} found"),
                    }),
                )
                    .into_response()
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericMessage {
                detail: "Error getting organization".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get all organizations
#[utoipa::path(
    get,
    path = (format!("{}/organization", Config::new(None).api_v1_prefix)),
    tag = "Organization",
    responses((status = 200, description = "Organization information", body = [Organization])),
)]
pub async fn get_organizations(State(pool): State<PgPool>) -> Response {
    match get_organizations_service(&pool).await {
        Ok(o) => (StatusCode::OK, Json(o)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericMessage {
                detail: "Error retrieving organizations".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Update an organization
#[utoipa::path(
    put,
    path = (format!("{}/organization", Config::new(None).api_v1_prefix)),
    request_body = OrganizationUpdate,
    tag = "Organization",
    responses((status = 200, description = "Organization added successfully", body = Organization)),
)]
pub async fn update_organization(
    State(pool): State<PgPool>,
    Json(new_organization): Json<OrganizationUpdate>,
) -> Response {
    match update_organization_service(&pool, &new_organization).await {
        Ok(o) => (StatusCode::OK, Json(o)).into_response(),
        Err(e) => {
            println!("{:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericMessage {
                    detail: "Error adding organization".to_string(),
                }),
            )
                .into_response()
        }
    }
}
