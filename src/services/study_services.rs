use anyhow::{bail, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::Utc;
use sqlx::postgres::PgPool;

use crate::{
    models::study::{Study, StudyCreate, StudyInDb, StudyUpdate},
    services::organization_services::get_organization_service,
};

pub async fn create_study_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    new_study: &StudyCreate,
) -> Result<Study> {
    let organization = match get_organization_service(db_pool, &new_study.organization_id).await {
        Ok(org) => {
            if let Some(o) = org {
                o
            } else {
                bail!(format!(
                    "No organization with id {} found",
                    &new_study.organization_id
                ));
            }
        }
        Err(_) => bail!("Error retrieving organization"),
    };

    let prepped_study = StudyInDb::prepare_create(
        new_study.study_id.clone(),
        new_study.study_name.clone(),
        new_study.study_description.clone(),
        new_study.organization_id.clone(),
    )
    .await?;

    let db_study = sqlx::query_as!(
        StudyInDb,
        r#"
            INSERT INTO studies (
                id,
                study_id,
                study_name,
                study_description,
                organization_id,
                date_added,
                date_modified
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id,
                study_id,
                study_name,
                study_description,
                organization_id,
                date_added,
                date_modified
        "#,
        prepped_study.id,
        prepped_study.study_id,
        prepped_study.study_name,
        prepped_study.study_description,
        prepped_study.organization_id,
        prepped_study.date_added,
        prepped_study.date_modified,
    )
    .fetch_one(db_pool)
    .await?;

    let study = Study {
        id: db_study.id,
        study_id: db_study.study_id,
        study_name: db_study.study_name,
        study_description: db_study.study_description,
        organization,
    };

    tracing::debug!("Adding study to cache");
    add_study_to_cache(valkey_pool, &study).await?;
    tracing::debug!("Study successfully saved to cache");

    Ok(study)
}

pub async fn delete_study_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    id: &str,
) -> Result<()> {
    let result = sqlx::query!(
        r#"
            DELETE FROM studies
            WHERE id = $1
        "#,
        id,
    )
    .execute(db_pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::debug!("Study successfully deleted from database");

        let mut conn = valkey_pool.get().await?;
        redis::cmd("DEL")
            .arg("studies")
            .arg(id)
            .query_async(&mut *conn)
            .await?;

        tracing::debug!("Study successfully deleted from cache");
        Ok(())
    } else {
        bail!(format!("No study with the id {id} found"));
    }
}

pub async fn get_study_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    study_id: &str,
    skip_cache: bool,
) -> Result<Option<Study>> {
    if !skip_cache {
        tracing::debug!("Checking for study in cache");
        let mut conn = valkey_pool.get().await?;
        let cached_study_str: Option<String> = redis::cmd("HGET")
            .arg("studies")
            .arg(study_id)
            .query_async(&mut *conn)
            .await?;

        match cached_study_str {
            Some(c) => {
                tracing::debug!("Study found in cache");
                let cached_study: Study = serde_json::from_str(&c)?;
                return Ok(Some(cached_study));
            }
            None => tracing::debug!("Study not found in cache"),
        }
    }

    tracing::debug!("Checking for study in database");
    let db_study = sqlx::query_as!(
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
        study_id,
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(s) = db_study {
        let organization = get_organization_service(db_pool, &s.organization_id).await;

        if let Ok(org) = organization {
            if let Some(o) = org {
                let study = Study {
                    id: s.id,
                    study_id: s.study_id,
                    study_name: s.study_name,
                    study_description: s.study_description,
                    organization: o,
                };

                tracing::debug!("Study found in database, adding to cache");
                add_study_to_cache(valkey_pool, &study).await?;
                tracing::debug!("Study successfully added to cache");

                Ok(Some(study))
            } else {
                bail!("No organization found for study");
            }
        } else {
            bail!("An error occurred retrieving the study: organization not found");
        }
    } else {
        Ok(None)
    }
}

pub async fn get_studies_service(db_pool: &PgPool) -> Result<Vec<Study>> {
    let db_studies = sqlx::query_as!(
        StudyInDb,
        r#"
            SELECT
                id,
                study_name,
                study_id,
                study_description,
                organization_id,
                date_added,
                date_modified
            FROM studies
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    let mut studies: Vec<Study> = Vec::new();

    for db_study in db_studies.into_iter() {
        let organization = get_organization_service(db_pool, &db_study.organization_id).await;

        if let Ok(org) = organization {
            if let Some(o) = org {
                let study = Study {
                    id: db_study.id,
                    study_id: db_study.study_id,
                    study_name: db_study.study_name,
                    study_description: db_study.study_description,
                    organization: o,
                };

                studies.push(study);
            } else {
                bail!("No organization found for study");
            }
        } else {
            bail!("An error occurred retrieving the study: organization not found");
        }
    }

    Ok(studies)
}

pub async fn update_study_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    updated_study: &StudyUpdate,
) -> Result<Study> {
    let organization = match get_organization_service(db_pool, &updated_study.organization_id).await
    {
        Ok(org) => {
            if let Some(o) = org {
                o
            } else {
                bail!("No organization found for study");
            }
        }
        Err(_) => bail!("Error retrieving organization"),
    };

    tracing::debug!("Updating study in database");
    let db_study = sqlx::query_as!(
        StudyInDb,
        r#"
            UPDATE studies
            SET
              study_id = $2,
              study_name = $3,
              study_description = $4,
              organization_id = $5,
              date_modified = $6
            WHERE id = $1
            RETURNING
                id,
                study_id,
                study_name,
                study_description,
                organization_id,
                date_added,
                date_modified
        "#,
        updated_study.id,
        updated_study.study_id,
        updated_study.study_name,
        updated_study.study_description,
        updated_study.organization_id,
        Utc::now(),
    )
    .fetch_one(db_pool)
    .await?;
    tracing::debug!("Successfully updated study in database");

    let study = Study {
        id: db_study.id,
        study_id: db_study.study_id,
        study_name: db_study.study_name,
        study_description: db_study.study_description,
        organization,
    };

    tracing::debug!("Adding updated study to cache");
    add_study_to_cache(valkey_pool, &study).await?;

    Ok(study)
}

async fn add_study_to_cache(pool: &Pool<RedisConnectionManager>, study: &Study) -> Result<()> {
    let study_json = serde_json::to_string(study)?;
    let mut conn = pool.get().await?;
    redis::cmd("HSET")
        .arg("studies")
        .arg(&study.id)
        .arg(study_json)
        .query_async(&mut *conn)
        .await?;

    Ok(())
}
