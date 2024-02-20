use anyhow::{bail, Result};
use chrono::Utc;
use sqlx::postgres::PgPool;

use crate::models::organization::{Organization, OrganizationCreate, OrganizationUpdate};

pub async fn create_organization(
    pool: &PgPool,
    new_organization: &OrganizationCreate,
) -> Result<Organization> {
    let organization = Organization::new(new_organization.name.clone());

    let added_org = sqlx::query_as!(
        Organization,
        r#"
            INSERT INTO organizations(id, name, active, date_added, date_modified)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, active, date_added, date_modified
        "#,
        organization.id,
        organization.name,
        organization.active,
        organization.date_added,
        organization.date_modified,
    )
    .fetch_one(pool)
    .await?;

    Ok(added_org)
}

pub async fn delete_organization(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query!(
        r#"
            DELETE FROM organizations
            WHERE id = $1
        "#,
        id,
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        Ok(())
    } else {
        bail!(format!("No organization with id {id} found"));
    }
}

pub async fn get_organization(pool: &PgPool, id: &str) -> Result<Organization> {
    let organization = sqlx::query_as!(
        Organization,
        r#"
            SELECT id, name, active, date_added, date_modified
            FROM organizations
            WHERE id = $1
        "#,
        id,
    )
    .fetch_one(pool)
    .await?;

    Ok(organization)
}

pub async fn get_organizations(pool: &PgPool) -> Result<Vec<Organization>> {
    let organizations = sqlx::query_as!(
        Organization,
        r#"
            SELECT id, name, active, date_added, date_modified
            FROM organizations
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(organizations)
}

pub async fn update_organization(
    pool: &PgPool,
    updated_organization: &OrganizationUpdate,
) -> Result<Organization> {
    let updated_org = sqlx::query_as!(
        Organization,
        r#"
            UPDATE organizations
            SET name = $2, active = $3, date_modified = $4
            WHERE id = $1
            RETURNING id, name, active, date_added, date_modified
        "#,
        updated_organization.id,
        updated_organization.name,
        updated_organization.active,
        Utc::now(),
    )
    .fetch_one(pool)
    .await?;

    Ok(updated_org)
}
