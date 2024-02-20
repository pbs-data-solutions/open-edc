use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

use crate::utils::generate_db_id;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub date_added: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

impl Organization {
    pub fn new(name: String) -> Self {
        Self {
            id: generate_db_id(),
            name,
            active: true,
            date_added: Utc::now(),
            date_modified: Utc::now(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OrganizationCreate {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OrganizationUpdate {
    pub id: String,
    pub name: String,
    pub active: bool,
}

pub async fn create_organization_service(
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

pub async fn delete_organization_service(pool: &PgPool, id: &str) -> Result<()> {
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

pub async fn get_organization_service(pool: &PgPool, id: &str) -> Result<Organization> {
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

pub async fn get_organizations_service(pool: &PgPool) -> Result<Vec<Organization>> {
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

pub async fn update_organization_service(
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
