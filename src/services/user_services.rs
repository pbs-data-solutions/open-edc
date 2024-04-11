use anyhow::{bail, Result};
use sqlx::postgres::PgPool;

use crate::{
    models::user::{User, UserCreate, UserInDb},
    services::organization_services::get_organization_service,
};

pub async fn create_user_service(pool: &PgPool, new_user: &UserCreate) -> Result<User> {
    let organization = match get_organization_service(pool, &new_user.organization_id).await {
        Ok(org) => {
            if let Some(o) = org {
                o
            } else {
                bail!("No organization found for user");
            }
        }
        Err(_) => bail!("Error retrieving organization"),
    };

    let prepped_user = UserInDb::prepare_create(
        new_user.user_name.to_string(),
        new_user.first_name.to_string(),
        new_user.last_name.to_string(),
        new_user.email.to_string(),
        new_user.password.to_string(),
        organization.id.clone(),
    )
    .await?;

    let db_user = sqlx::query_as!(
        UserInDb,
        r#"
            INSERT INTO users (
                id,
                user_name,
                first_name,
                last_name,
                email,
                hashed_password,
                organization_id,
                active,
                date_added,
                date_modified
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING
                id,
                user_name,
                first_name,
                last_name,
                email,
                hashed_password,
                organization_id,
                active,
                date_added,
                date_modified
        "#,
        prepped_user.id,
        prepped_user.user_name,
        prepped_user.first_name,
        prepped_user.last_name,
        prepped_user.email,
        prepped_user.hashed_password,
        prepped_user.organization_id,
        prepped_user.active,
        prepped_user.date_added,
        prepped_user.date_modified,
    )
    .fetch_one(pool)
    .await?;

    let user = User {
        id: db_user.id,
        user_name: db_user.user_name,
        first_name: db_user.first_name,
        last_name: db_user.last_name,
        email: db_user.email,
        organization,
        active: db_user.active,
    };

    Ok(user)
}

pub async fn get_user_service(pool: &PgPool, user_id: &str) -> Result<Option<User>> {
    let db_user = sqlx::query_as!(
        UserInDb,
        r#"
            SELECT
                id,
                user_name,
                first_name,
                last_name,
                email,
                hashed_password,
                active,
                organization_id,
                date_added,
                date_modified
            FROM users
            WHERE id = $1
        "#,
        user_id,
    )
    .fetch_optional(pool)
    .await?;

    if let Some(u) = db_user {
        let organization = get_organization_service(pool, &u.organization_id).await;

        if let Ok(org) = organization {
            if let Some(o) = org {
                let user = User {
                    id: u.id,
                    user_name: u.user_name,
                    first_name: u.first_name,
                    last_name: u.last_name,
                    email: u.email,
                    active: u.active,
                    organization: o,
                };

                Ok(Some(user))
            } else {
                bail!("No organization found for user");
            }
        } else {
            bail!("An error occurred retrieving the user: organization not found");
        }
    } else {
        Ok(None)
    }
}

pub async fn get_users_service(pool: &PgPool) -> Result<Vec<User>> {
    let db_users = sqlx::query_as!(
        UserInDb,
        r#"
            SELECT
                id,
                user_name,
                first_name,
                last_name,
                email,
                hashed_password,
                active,
                organization_id,
                date_added,
                date_modified
            FROM users
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut users: Vec<User> = Vec::new();

    for db_user in db_users.into_iter() {
        let organization = get_organization_service(pool, &db_user.organization_id).await;

        if let Ok(org) = organization {
            if let Some(o) = org {
                let user = User {
                    id: db_user.id,
                    user_name: db_user.user_name,
                    first_name: db_user.first_name,
                    last_name: db_user.last_name,
                    email: db_user.email,
                    active: db_user.active,
                    organization: o,
                };

                users.push(user);
            } else {
                bail!("No organization found for user");
            }
        } else {
            bail!("An error occurred retrieving the user: organization not found");
        }
    }

    Ok(users)
}

#[allow(dead_code)]
pub async fn get_organization_users_service() {}
