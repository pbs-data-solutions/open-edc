use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    models::{organization::Organization, study::Study},
    utils::{generate_db_id, hash_password},
};

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum AccessLevel {
    OrganizationAdmin,
    SystemAdmin,
    User,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UserInDb {
    pub id: String,
    pub user_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub hashed_password: String,
    pub organization_id: String,
    pub active: bool,
    pub access_level: AccessLevel,
    pub date_added: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

impl UserInDb {
    pub async fn prepare_create(
        user_name: String,
        first_name: String,
        last_name: String,
        email: String,
        password: String,
        organization_id: String,
    ) -> Result<Self> {
        let hashed_password = hash_password(&password).await?;
        Ok(Self {
            id: generate_db_id(),
            user_name,
            first_name,
            last_name,
            email,
            hashed_password,
            organization_id,
            active: true,
            access_level: AccessLevel::User,
            date_added: Utc::now(),
            date_modified: Utc::now(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct User {
    /// Uniue system identifier for the user
    pub id: String,
    pub user_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub organization: Organization,
    pub studies: Option<Vec<Study>>,
    pub active: bool,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserCreate {
    pub user_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub organization_id: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserUpdate {
    /// Uniue system identifier for the user
    pub id: String,
    pub user_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: Option<String>,
    pub active: bool,
    pub organization_id: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserStudy {
    /// User's unique system identifier
    pub user_id: String,

    /// Study's unique system identifier
    pub study_id: String,
}
