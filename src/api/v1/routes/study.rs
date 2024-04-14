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
    models::messages::GenericMessage,
    models::study::{StudyCreate, StudyUpdate},
    services::study_services::{
        create_study_service, delete_study_service, get_studies_service, get_study_service,
        update_study_service,
    },
};

pub fn study_routes(pool: PgPool, config: &Config) -> Router<PgPool> {
    let prefix = format!("{}/study", config.api_v1_prefix);
    Router::new()
        .route(&prefix, post(create_study))
        .with_state(pool.clone())
        .route(&format!("{prefix}/:id"), delete(delete_study))
        .with_state(pool.clone())
        .route(&format!("{prefix}/:id"), get(get_study))
        .with_state(pool.clone())
        .route(&prefix, get(get_studies))
        .with_state(pool.clone())
        // TODO: I want to make this a patch but need to figure out how to diferentiate between
        // default None and study set None in serde.
        .route(&prefix, put(update_study))
        .with_state(pool.clone())
}

/// Create a new study
#[utoipa::path(
    post,
    path = (format!("{}/study", Config::new(None).api_v1_prefix)),
    request_body = StudyCreate,
    tag = "Studies",
    responses(
        (status = 200, description = "Study added successfully", body = Study),
        (status = 400, body = GenericMessage)
    )
)]
pub async fn create_study(
    State(pool): State<PgPool>,
    Json(new_study): Json<StudyCreate>,
) -> Response {
    tracing::debug!("Creating study");

    match create_study_service(&pool, &new_study).await {
        Ok(study) => {
            tracing::debug!("Successfully created study");
            (StatusCode::CREATED, Json(study)).into_response()
        }
        Err(e) => {
            tracing::error!("Error creating study: {}", e.to_string());

            if e.to_string().contains("violates unique constraint") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!(
                            "An study with the study id {} already exists",
                            &new_study.study_id
                        ),
                    }),
                )
                    .into_response()
            } else if e.to_string().contains("No organization found") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!("Organization id {} not found", &new_study.organization_id),
                    }),
                )
                    .into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenericMessage {
                        detail: "An error occurred while creating study".to_string(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

/// Delete a study by database id
#[utoipa::path(
    delete,
    path = (format!("{}/study/{{id}}", Config::new(None).api_v1_prefix)),
    params(
        ("id" = String, Path, description = "Study database id")
    ),
    tag = "Studies",
    responses(
        (status = 204, description = "Study successfully deleted"),
        (status = 404, description = "Study not found", body = GenericMessage),
    )
)]
pub async fn delete_study(State(pool): State<PgPool>, Path(id): Path<String>) -> Response {
    tracing::debug!("Deleting study {id}");

    match delete_study_service(&pool, &id).await {
        Ok(o) => {
            tracing::debug!("Successfully deleted study {id}");
            (StatusCode::NO_CONTENT, Json(o)).into_response()
        }
        Err(e) => {
            tracing::error!("Error deleting study: {}", e.to_string());

            if e.to_string().contains("No study with the id") {
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
                        detail: "Error deleting study".to_string(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

/// Get a study by database id
#[utoipa::path(
    get,
    path = (format!("{}/study/{{id}}", Config::new(None).api_v1_prefix)),
    tag = "Studies",
    responses(
        (status = 200, description = "Study information", body = Study),
        (status = 404, description = "Study not found", body = GenericMessage)
    )
)]
pub async fn get_study(State(pool): State<PgPool>, Path(id): Path<String>) -> Response {
    tracing::debug!("Getting study {id}");

    match get_study_service(&pool, &id).await {
        Ok(study) => {
            if let Some(s) = study {
                tracing::debug!("Successfully retrieved study {id}");
                (StatusCode::OK, Json(s)).into_response()
            } else {
                tracing::error!("Study {id} not found");
                (
                    StatusCode::NOT_FOUND,
                    Json(GenericMessage {
                        detail: format!("No study with id {id} found"),
                    }),
                )
                    .into_response()
            }
        }
        Err(e) => {
            tracing::error!("Error retrieving study {id}: {}", e.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericMessage {
                    detail: "Error getting study".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Get all study
#[utoipa::path(
    get,
    path = (format!("{}/study", Config::new(None).api_v1_prefix)),
    tag = "Studies",
    responses(
        (status = 200, description = "All studies information", body = [Study]),
    )
)]
pub async fn get_studies(State(pool): State<PgPool>) -> Response {
    tracing::debug!("Getting all studies");

    match get_studies_service(&pool).await {
        Ok(u) => {
            tracing::debug!("Successfully retrieved all studies");
            (StatusCode::OK, Json(u)).into_response()
        }
        Err(e) => {
            tracing::error!("Error retrieving all studies: {}", e.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericMessage {
                    detail: "Error retrieving studies".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Update a study by database id
#[utoipa::path(
    put,
    path = (format!("{}/study", Config::new(None).api_v1_prefix)),
    request_body = StudyUpdate,
    tag = "Studies",
    responses((status = 200, description = "Study added successfully", body = Organization)),
    responses((status = 400, body = GenericMessage)),
)]
pub async fn update_study(
    State(pool): State<PgPool>,
    Json(study_update): Json<StudyUpdate>,
) -> Response {
    tracing::debug!("Updating study");

    match update_study_service(&pool, &study_update).await {
        Ok(o) => {
            tracing::debug!("Successfully updated study");
            (StatusCode::OK, Json(o)).into_response()
        }
        Err(e) => {
            tracing::error!("Error updating study: {}", e.to_string());

            if e.to_string().contains("violates unique constraint") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!(
                            "An study with the study id {} already exists",
                            &study_update.study_id
                        ),
                    }),
                )
                    .into_response()
            } else if e.to_string().contains("No organization found") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!(
                            "Organization id {} not found",
                            &study_update.organization_id
                        ),
                    }),
                )
                    .into_response()
            } else if e.to_string().contains("no rows returned") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!("No study with id {} found", &study_update.id),
                    }),
                )
                    .into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenericMessage {
                        detail: "Error adding study".to_string(),
                    }),
                )
                    .into_response()
            }
        }
    }
}
