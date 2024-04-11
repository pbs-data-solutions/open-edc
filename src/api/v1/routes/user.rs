use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use sqlx::postgres::PgPool;

use crate::{
    config::Config,
    models::messages::GenericMessage,
    models::user::{create_user_service, get_user_service, get_users_service, UserCreate},
};

pub fn user_routes(pool: PgPool, config: &Config) -> Router<PgPool> {
    let prefix = format!("{}/user", config.api_v1_prefix);
    Router::new()
        .route(&prefix, post(create_user))
        .with_state(pool.clone())
        // .route(&format!("{prefix}/:id"), delete(delete_organization))
        // .with_state(pool.clone())
        .route(&format!("{prefix}/:id"), get(get_user))
        .with_state(pool.clone())
        .route(&prefix, get(get_users))
        .with_state(pool.clone())
    // TODO: I want to make this a patch but need to figure out how to diferentiate between
    // default None and user set None in serde.
    // .route(&prefix, put(update_org))
    // .with_state(pool.clone())
}

pub async fn create_user(State(pool): State<PgPool>, Json(new_user): Json<UserCreate>) -> Response {
    match create_user_service(&pool, &new_user).await {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(e) => {
            if e.to_string().contains("violates unique constraint") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(GenericMessage {
                        detail: format!(
                            "An user with the user name {} already exists",
                            &new_user.user_name
                        ),
                    }),
                )
                    .into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenericMessage {
                        detail: "An error occurred while creating user".to_string(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

pub async fn get_user(State(pool): State<PgPool>, Path(id): Path<String>) -> Response {
    match get_user_service(&pool, &id).await {
        Ok(user) => {
            if let Some(u) = user {
                (StatusCode::OK, Json(u)).into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(GenericMessage {
                        detail: format!("No user with id {id} found"),
                    }),
                )
                    .into_response()
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericMessage {
                detail: "Error getting user".to_string(),
            }),
        )
            .into_response(),
    }
}

pub async fn get_users(State(pool): State<PgPool>) -> Response {
    match get_users_service(&pool).await {
        Ok(u) => (StatusCode::OK, Json(u)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericMessage {
                detail: "Error retrieving users".to_string(),
            }),
        )
            .into_response(),
    }
}
