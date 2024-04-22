use anyhow::{bail, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::Utc;
use sqlx::postgres::PgPool;

use crate::{
    models::organization::{Organization, OrganizationCreate, OrganizationUpdate},
    services::cache_services::{add_cached_value, delete_cached_value, get_cached_value},
};

pub async fn create_organization_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
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
    .fetch_one(db_pool)
    .await?;

    tracing::debug!("Adding organization to cache");
    add_cached_value(valkey_pool, "organizations", &added_org.id, &organization).await?;
    tracing::debug!("Organization successfully saved to cache");

    Ok(added_org)
}

pub async fn delete_organization_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    organization_id: &str,
) -> Result<()> {
    let result = sqlx::query!(
        r#"
            DELETE FROM organizations
            WHERE id = $1
        "#,
        organization_id,
    )
    .execute(db_pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::debug!("Organization successfully deleted from database, deleting from cache");

        delete_cached_value(valkey_pool, "organizations", organization_id).await?;
        tracing::debug!("Organization successfully deleted from cache");
        Ok(())
    } else {
        bail!(format!(
            "No organization with the id {organization_id} found"
        ));
    }
}

pub async fn get_organization_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    organization_id: &str,
    skip_cache: bool,
) -> Result<Option<Organization>> {
    if !skip_cache {
        tracing::debug!("Checking for organization in cache");
        let cached_organization =
            get_cached_value(valkey_pool, "organizations", organization_id).await?;
        if cached_organization.is_some() {
            return Ok(cached_organization);
        } else {
            tracing::debug!("Organization not found in cache");
        }
    }
    let organization = sqlx::query_as!(
        Organization,
        r#"
            SELECT id, name, active, date_added, date_modified
            FROM organizations
            WHERE id = $1
        "#,
        organization_id,
    )
    .fetch_optional(db_pool)
    .await?;

    Ok(organization)
}

pub async fn get_organizations_service(db_pool: &PgPool) -> Result<Vec<Organization>> {
    let organizations = sqlx::query_as!(
        Organization,
        r#"
            SELECT id, name, active, date_added, date_modified
            FROM organizations
        "#
    )
    .fetch_all(db_pool)
    .await?;

    Ok(organizations)
}

pub async fn update_organization_service(
    db_pool: &PgPool,
    valkey_pool: &Pool<RedisConnectionManager>,
    updated_organization: &OrganizationUpdate,
) -> Result<Organization> {
    tracing::debug!("Updating organization in database");
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
    .fetch_one(db_pool)
    .await?;
    tracing::debug!("Successfully updated organization in database");

    tracing::debug!("Adding updated organization to cache");
    add_cached_value(valkey_pool, "organizations", &updated_org.id, &updated_org).await?;

    Ok(updated_org)
}
