use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{models::organization::Organization, utils::generate_db_id};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StudyInDb {
    pub id: String,
    pub study_id: String,
    pub study_name: Option<String>,
    pub study_description: Option<String>,
    pub organization_id: String,
    pub date_added: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

impl StudyInDb {
    pub async fn prepare_create(
        study_id: String,
        study_name: Option<String>,
        study_description: Option<String>,
        organization_id: String,
    ) -> Result<Self> {
        Ok(Self {
            id: generate_db_id(),
            study_id,
            study_name,
            study_description,
            organization_id,
            date_added: Utc::now(),
            date_modified: Utc::now(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Study {
    /// Uniue system identifier for the study
    pub id: String,
    pub study_id: String,
    pub study_name: Option<String>,
    pub study_description: Option<String>,
    pub organization: Organization,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct StudyCreate {
    pub study_id: String,
    pub study_name: Option<String>,
    pub study_description: Option<String>,
    pub organization_id: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct StudyUpdate {
    /// Uniue system identifier for the study
    pub id: String,
    pub study_id: String,
    pub study_name: Option<String>,
    pub study_description: Option<String>,
    pub organization_id: String,
}
