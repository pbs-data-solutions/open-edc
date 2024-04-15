use anyhow::{bail, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::Utc;
use sqlx::postgres::PgPool;

use crate::{
    models::{
        study::{Study, StudyInDb},
        user::{AccessLevel, User, UserCreate, UserInDb, UserUpdate},
    },
    services::{
        organization_services::get_organization_service, study_services::get_study_service,
    },
    utils::{generate_db_id, hash_password},
};

pub async fn add_user_to_study_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    user_id: &str,
    study_id: &str,
) -> Result<User> {
    let user_org =
        if let Some(user) = get_user_service(db_pool, valkey_pool, user_id, false).await? {
            user.organization.id
        } else {
            bail!(format!("No user with id {user_id} found"));
        };
    let study_org = if let Some(study) = get_study_service(db_pool, study_id).await? {
        study.organization.id
    } else {
        bail!(format!("No study with id {study_id} found"));
    };

    if user_org != study_org {
        bail!("Study id {study_id} not found");
    }

    let db_id = generate_db_id();

    tracing::debug!("Adding user to study in database");
    sqlx::query!(
        r#"
            INSERT INTO user_studies (
                id,
                user_id,
                study_id,
                date_added,
                date_modified
            )
            VALUES ($1, $2, $3, $4, $5)
        "#,
        db_id,
        user_id,
        study_id,
        Utc::now(),
        Utc::now(),
    )
    .execute(db_pool)
    .await?;

    if let Some(user) = get_user_service(db_pool, valkey_pool, user_id, true).await? {
        tracing::debug!("User successfully added to study in database, updating cache");
        add_user_to_cache(valkey_pool, &user).await?;
        tracing::debug!("Cache successfully updated");
        Ok(user)
    } else {
        bail!("Error retrieving user");
    }
}

pub async fn create_user_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    new_user: &UserCreate,
) -> Result<User> {
    let organization = match get_organization_service(db_pool, &new_user.organization_id).await {
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
                access_level,
                date_added,
                date_modified
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id,
                user_name,
                first_name,
                last_name,
                email,
                hashed_password,
                active,
                organization_id,
                access_level AS "access_level: AccessLevel",
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
        prepped_user.access_level as AccessLevel,
        prepped_user.date_added,
        prepped_user.date_modified,
    )
    .fetch_one(db_pool)
    .await?;

    tracing::debug!("User successfully saved to database");

    let studies = get_user_studies_service(db_pool, &db_user.id).await?;
    let user = User {
        id: db_user.id,
        user_name: db_user.user_name,
        first_name: db_user.first_name,
        last_name: db_user.last_name,
        email: db_user.email,
        organization,
        studies,
        active: db_user.active,
    };

    tracing::debug!("Adding user to cache");
    add_user_to_cache(valkey_pool, &user).await?;
    tracing::debug!("User successfully saved to cache");

    Ok(user)
}

pub async fn delete_user_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    id: &str,
) -> Result<()> {
    let result = sqlx::query!(
        r#"
            DELETE FROM users
            WHERE id = $1
        "#,
        id,
    )
    .execute(db_pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::debug!("User successfully deleted from database");

        let mut conn = valkey_pool.get().await?;
        redis::cmd("DEL")
            .arg("users")
            .arg(id)
            .query_async(&mut *conn)
            .await?;

        tracing::debug!("User successfully deleted from cache");
        Ok(())
    } else {
        bail!(format!("No user with the id {id} found"));
    }
}

pub async fn get_user_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    user_id: &str,
    skip_cache: bool,
) -> Result<Option<User>> {
    if !skip_cache {
        tracing::debug!("Checking for user in cache");
        let mut conn = valkey_pool.get().await?;
        let cached_user_str: Option<String> = redis::cmd("HGET")
            .arg("users")
            .arg(user_id)
            .query_async(&mut *conn)
            .await?;

        match cached_user_str {
            Some(c) => {
                tracing::debug!("User found in cache");
                let cached_user: User = serde_json::from_str(&c)?;
                return Ok(Some(cached_user));
            }
            None => tracing::debug!("User not found in cache"),
        }
    }

    tracing::debug!("Checking for user in database");
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
                organization_id,
                active,
                access_level AS "access_level: AccessLevel",
                date_added,
                date_modified
            FROM users
            WHERE id = $1
        "#,
        user_id,
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(u) = db_user {
        let organization = get_organization_service(db_pool, &u.organization_id).await;
        let studies = get_user_studies_service(db_pool, &u.id).await?;

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
                    studies,
                };

                tracing::debug!("User found in database, adding to cache");
                add_user_to_cache(valkey_pool, &user).await?;
                tracing::debug!("User successfully added to cache");
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

pub async fn get_user_studies_service(
    db_pool: &PgPool,
    user_id: &str,
) -> Result<Option<Vec<Study>>> {
    let db_studies: Vec<StudyInDb> = sqlx::query_as!(
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
            WHERE id in (SELECT study_id FROM user_studies WHERE user_id = $1)
        "#,
        user_id,
    )
    .fetch_all(db_pool)
    .await?;

    if !db_studies.is_empty() {
        let organization =
            match get_organization_service(db_pool, &db_studies[0].organization_id).await {
                Ok(org) => {
                    if let Some(o) = org {
                        o
                    } else {
                        bail!("No organization found for user");
                    }
                }
                Err(_) => bail!("Error retrieving organization"),
            };
        let mut studies: Vec<Study> = Vec::new();
        for study in db_studies.into_iter() {
            let s = Study {
                id: study.id,
                study_id: study.study_id,
                study_name: study.study_name,
                study_description: study.study_description,
                organization: organization.clone(),
            };
            studies.push(s);
        }
        Ok(Some(studies))
    } else {
        Ok(None)
    }
}

pub async fn get_users_service(db_pool: &PgPool) -> Result<Vec<User>> {
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
                organization_id,
                active,
                access_level AS "access_level: AccessLevel",
                date_added,
                date_modified
            FROM users
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    let mut users: Vec<User> = Vec::new();

    for db_user in db_users.into_iter() {
        let organization = get_organization_service(db_pool, &db_user.organization_id).await;
        let studies = get_user_studies_service(db_pool, &db_user.id).await?;

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
                    studies,
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

pub async fn remove_user_from_study_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    user_id: &str,
    study_id: &str,
) -> Result<()> {
    tracing::debug!("Removing use from database");
    let result = sqlx::query!(
        r#"
            DELETE FROM user_studies
            where user_id = $1 and study_id = $2
        "#,
        user_id,
        study_id,
    )
    .execute(db_pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::debug!("successfully removed user from database, updating cache");
        match get_user_service(db_pool, valkey_pool, user_id, true).await {
            Ok(user) => match user {
                Some(u) => {
                    add_user_to_cache(valkey_pool, &u).await?;
                    tracing::debug!("Cache successfully updated");
                }
                None => tracing::debug!("Error updating cache, user not found"),
            },
            Err(e) => {
                tracing::error!("Error adding user to cache: {}", e.to_string());
            }
        }
        Ok(())
    } else {
        bail!(format!(
            "No user with the id {user_id} and study id {study_id} found"
        ));
    }
}

pub async fn update_user_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    updated_user: &UserUpdate,
) -> Result<User> {
    let organization = match get_organization_service(db_pool, &updated_user.organization_id).await
    {
        Ok(org) => {
            if let Some(o) = org {
                o
            } else {
                bail!("No organization found for user");
            }
        }
        Err(_) => bail!("Error retrieving organization"),
    };

    let studies = get_user_studies_service(db_pool, &updated_user.id).await?;

    tracing::debug!("Updating user in database");
    let db_user = if let Some(password) = &updated_user.password {
        let hashed_password = hash_password(password).await?;
        sqlx::query_as!(
            UserInDb,
            r#"
                UPDATE users
                SET
                  user_name = $2,
                  first_name = $3,
                  last_name = $4,
                  email = $5,
                  hashed_password = $6,
                  active = $7,
                  organization_id = $8,
                  date_modified = $9
                WHERE id = $1
                RETURNING
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
            "#,
            updated_user.id,
            updated_user.user_name,
            updated_user.first_name,
            updated_user.last_name,
            updated_user.email,
            hashed_password,
            updated_user.active,
            updated_user.organization_id,
            Utc::now(),
        )
        .fetch_one(db_pool)
        .await?
    } else {
        sqlx::query_as!(
            UserInDb,
            r#"
                UPDATE users
                SET
                  user_name = $2,
                  first_name = $3,
                  last_name = $4,
                  email = $5,
                  active = $6,
                  organization_id = $7,
                  date_modified = $8
                WHERE id = $1
                RETURNING
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
            "#,
            updated_user.id,
            updated_user.user_name,
            updated_user.first_name,
            updated_user.last_name,
            updated_user.email,
            updated_user.active,
            updated_user.organization_id,
            Utc::now(),
        )
        .fetch_one(db_pool)
        .await?
    };
    tracing::debug!("Successfully updated user in database");

    let user = User {
        id: db_user.id,
        user_name: db_user.user_name,
        first_name: db_user.first_name,
        last_name: db_user.last_name,
        email: db_user.email,
        organization,
        studies,
        active: db_user.active,
    };

    tracing::debug!("Adding updated user to cache");
    add_user_to_cache(valkey_pool, &user).await?;

    Ok(user)
}

async fn add_user_to_cache(pool: &Pool<RedisConnectionManager>, user: &User) -> Result<()> {
    let user_json = serde_json::to_string(user)?;
    let mut conn = pool.get().await?;
    redis::cmd("HSET")
        .arg("users")
        .arg(&user.id)
        .arg(user_json)
        .query_async(&mut *conn)
        .await?;

    Ok(())
}
