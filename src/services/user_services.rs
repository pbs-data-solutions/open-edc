use anyhow::{bail, Result};
use chrono::Utc;
use sqlx::postgres::PgPool;

use crate::{
    models::{
        study::{Study, StudyInDb},
        user::{User, UserCreate, UserInDb, UserUpdate},
    },
    services::{
        organization_services::get_organization_service, study_services::get_study_service,
    },
    utils::{generate_db_id, hash_password},
};

pub async fn add_user_to_study_service(
    pool: &PgPool,
    user_id: &str,
    study_id: &str,
) -> Result<User> {
    let user_org = if let Some(user) = get_user_service(pool, user_id).await? {
        user.organization.id
    } else {
        bail!(format!("No user with id {user_id} found"));
    };
    let study_org = if let Some(study) = get_study_service(pool, study_id).await? {
        study.organization.id
    } else {
        bail!(format!("No study with id {study_id} found"));
    };

    if user_org != study_org {
        bail!("Study id {study_id} not found");
    }

    let db_id = generate_db_id();

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
    .execute(pool)
    .await?;

    if let Some(user) = get_user_service(pool, user_id).await? {
        Ok(user)
    } else {
        bail!("Error retrieving user");
    }
}

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

    let studies = get_user_studies_service(pool, &db_user.id).await?;

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

    Ok(user)
}

pub async fn delete_user_service(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query!(
        r#"
            DELETE FROM users
            WHERE id = $1
        "#,
        id,
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        Ok(())
    } else {
        bail!(format!("No user with the id {id} found"));
    }
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
        let studies = get_user_studies_service(pool, &u.id).await?;

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

pub async fn get_user_studies_service(pool: &PgPool, user_id: &str) -> Result<Option<Vec<Study>>> {
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
    .fetch_all(pool)
    .await?;

    if !db_studies.is_empty() {
        let organization =
            match get_organization_service(pool, &db_studies[0].organization_id).await {
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
        let studies = get_user_studies_service(pool, &db_user.id).await?;

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
    pool: &PgPool,
    user_id: &str,
    study_id: &str,
) -> Result<()> {
    let result = sqlx::query!(
        r#"
            DELETE FROM user_studies
            where user_id = $1 and study_id = $2
        "#,
        user_id,
        study_id,
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        Ok(())
    } else {
        bail!(format!(
            "No user with the id {user_id} and study id {study_id} found"
        ));
    }
}

pub async fn update_user_service(pool: &PgPool, updated_user: &UserUpdate) -> Result<User> {
    let organization = match get_organization_service(pool, &updated_user.organization_id).await {
        Ok(org) => {
            if let Some(o) = org {
                o
            } else {
                bail!("No organization found for user");
            }
        }
        Err(_) => bail!("Error retrieving organization"),
    };

    let studies = get_user_studies_service(pool, &updated_user.id).await?;
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
                    active,
                    organization_id,
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
        .fetch_one(pool)
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
        .fetch_one(pool)
        .await?
    };

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

    Ok(user)
}
