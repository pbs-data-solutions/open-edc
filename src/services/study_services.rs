use anyhow::{bail, Result};
use chrono::Utc;
use sqlx::postgres::PgPool;

use crate::{
    models::study::{Study, StudyCreate, StudyInDb, StudyUpdate},
    services::organization_services::get_organization_service,
};

pub async fn create_study_service(pool: &PgPool, new_study: &StudyCreate) -> Result<Study> {
    let organization = match get_organization_service(pool, &new_study.organization_id).await {
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
    .fetch_one(pool)
    .await?;

    let study = Study {
        id: db_study.id,
        study_id: db_study.study_id,
        study_name: db_study.study_name,
        study_description: db_study.study_description,
        organization,
    };

    Ok(study)
}

pub async fn delete_study_service(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query!(
        r#"
            DELETE FROM studies
            WHERE id = $1
        "#,
        id,
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        Ok(())
    } else {
        bail!(format!("No study with the id {id} found"));
    }
}

pub async fn get_study_service(pool: &PgPool, study_id: &str) -> Result<Option<Study>> {
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
    .fetch_optional(pool)
    .await?;

    if let Some(s) = db_study {
        let organization = get_organization_service(pool, &s.organization_id).await;

        if let Ok(org) = organization {
            if let Some(o) = org {
                let study = Study {
                    id: s.id,
                    study_id: s.study_id,
                    study_name: s.study_name,
                    study_description: s.study_description,
                    organization: o,
                };

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

pub async fn get_studies_service(pool: &PgPool) -> Result<Vec<Study>> {
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
    .fetch_all(pool)
    .await?;

    let mut studies: Vec<Study> = Vec::new();

    for db_study in db_studies.into_iter() {
        let organization = get_organization_service(pool, &db_study.organization_id).await;

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

pub async fn update_study_service(pool: &PgPool, updated_study: &StudyUpdate) -> Result<Study> {
    let organization = match get_organization_service(pool, &updated_study.organization_id).await {
        Ok(org) => {
            if let Some(o) = org {
                o
            } else {
                bail!("No organization found for study");
            }
        }
        Err(_) => bail!("Error retrieving organization"),
    };

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
    .fetch_one(pool)
    .await?;

    let study = Study {
        id: db_study.id,
        study_id: db_study.study_id,
        study_name: db_study.study_name,
        study_description: db_study.study_description,
        organization,
    };

    Ok(study)
}
