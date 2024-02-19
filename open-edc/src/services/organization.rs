use anyhow::Result;
use sqlx::postgres::PgPool;

use crate::models::organization::{Organization, OrganizationCreate};

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
