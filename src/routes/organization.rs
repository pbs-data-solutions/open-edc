use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};

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
    state::AppState,
};

pub fn organization_routes(state: Arc<AppState>, config: &Config) -> Router<Arc<AppState>> {
    let prefix = format!("{}/organization", config.api_prefix);
    Router::new()
        .route(&prefix, post(create_organization))
        .with_state(state.clone())
        .route(&format!("{prefix}/:id"), delete(delete_organization))
        .with_state(state.clone())
        .route(&format!("{prefix}/:id"), get(get_organization))
        .with_state(state.clone())
        .route(&prefix, get(get_organizations))
        .with_state(state.clone())
        // TODO: I want to make this a patch but need to figure out how to diferentiate between
        // default None and user set None in serde.
        .route(&prefix, put(update_organization))
        .with_state(state.clone())
}

/// Add a new organization
#[utoipa::path(
    post,
    path = (format!("{}/organization", Config::new().api_prefix)),
    request_body = OrganizationCreate,
    tag = "Organizations",
    responses(
        (status = 200, description = "Organization added successfully", body = OrganizationCreate),
        (status = 400, description = "Organization already exists", body = GenericMessage)
    )
)]
pub async fn create_organization(
    State(state): State<Arc<AppState>>,
    Json(new_organization): Json<OrganizationCreate>,
) -> Response {
    tracing::debug!("Creating new organization");
    let db_pool = state.db_state.pool.clone();
    let valkey_pool = &state.valkey_state.pool;

    match create_organization_service(&db_pool, valkey_pool, &new_organization).await {
        Ok(o) => {
            tracing::debug!("Organization successfully created");
            (StatusCode::OK, Json(o)).into_response()
        }
        Err(e) => {
            tracing::error!("Error creating organization: {}", e.to_string());

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
    path = (format!("{}/organization/{{id}}", Config::new().api_prefix)),
    params(
        ("id" = String, Path, description = "Organization database id")
    ),
    tag = "Organizations",
    responses(
        (status = 204, description = "Organization successfully deleted"),
        (status = 404, description = "Organization not found", body = GenericMessage),
    )
)]
pub async fn delete_organization(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    tracing::debug!("Deleting organization {id}");
    let db_pool = state.db_state.pool.clone();
    let valkey_pool = &state.valkey_state.pool;

    match delete_organization_service(&db_pool, valkey_pool, &id).await {
        Ok(o) => {
            tracing::debug!("Successfully deleted organization {id}");
            (StatusCode::NO_CONTENT, Json(o)).into_response()
        }
        Err(e) => {
            tracing::error!("Error deleting organization {id}: {}", e.to_string());

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
    path = (format!("{}/organization/{{id}}", Config::new().api_prefix)),
    params(
        ("id" = String, Path, description = "Organization database id")
    ),
    tag = "Organizations",
    responses(
        (status = 200, description = "Organization information", body = Organization),
        (status = 404, description = "Organization not found", body = GenericMessage),
    )
)]
pub async fn get_organization(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    tracing::debug!("Getting organization {id}");
    let db_pool = state.db_state.pool.clone();
    let valkey_pool = &state.valkey_state.pool;

    match get_organization_service(&db_pool, valkey_pool, &id, false).await {
        Ok(organization) => {
            if let Some(o) = organization {
                tracing::debug!("Successfully retrieved organization {id}");
                (StatusCode::OK, Json(o)).into_response()
            } else {
                tracing::debug!("Organization {id} not found");
                (
                    StatusCode::NOT_FOUND,
                    Json(GenericMessage {
                        detail: format!("No organization with the id {id} found"),
                    }),
                )
                    .into_response()
            }
        }
        Err(e) => {
            tracing::error!("Error getting organization {id}: {}", e.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericMessage {
                    detail: "Error getting organization".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Get all organizations
#[utoipa::path(
    get,
    path = (format!("{}/organization", Config::new().api_prefix)),
    tag = "Organizations",
    responses((status = 200, description = "Organization information", body = [Organization])),
)]
pub async fn get_organizations(State(state): State<Arc<AppState>>) -> Response {
    tracing::debug!("Getting all organizations");
    let db_pool = state.db_state.pool.clone();

    match get_organizations_service(&db_pool).await {
        Ok(o) => {
            tracing::debug!("Successfully retrieved all organizaiton");
            (StatusCode::OK, Json(o)).into_response()
        }
        Err(e) => {
            tracing::error!("Error retrieving all organizations: {}", e.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericMessage {
                    detail: "Error retrieving organizations".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Update an organization
#[utoipa::path(
    put,
    path = (format!("{}/organization", Config::new().api_prefix)),
    request_body = OrganizationUpdate,
    tag = "Organizations",
    responses((status = 200, description = "Organization added successfully", body = Organization)),
)]
pub async fn update_organization(
    State(state): State<Arc<AppState>>,
    Json(update_organization): Json<OrganizationUpdate>,
) -> Response {
    tracing::debug!("Updating organization");
    let db_pool = state.db_state.pool.clone();
    let valkey_pool = &state.valkey_state.pool;

    match update_organization_service(&db_pool, valkey_pool, &update_organization).await {
        Ok(o) => {
            tracing::debug!("Successfully updated organization");
            (StatusCode::OK, Json(o)).into_response()
        }
        Err(e) => {
            tracing::error!("Error updating organization: {}", e.to_string());

            if e.to_string().contains("no rows returned") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!(
                            "No organization with id {} found",
                            &update_organization.id
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
