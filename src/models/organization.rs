use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils::generate_db_id;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(rename_all = "camelCase")]
pub struct Organization {
    /// Uniue system identifier for the organization
    pub id: String,

    /// The name of of the organization
    pub name: String,

    /// Is the organization activate
    pub active: bool,

    /// Date the organization was added
    pub date_added: DateTime<Utc>,

    /// Date the orginization was last modified
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(rename_all = "camelCase")]
pub struct OrganizationCreate {
    /// The name of of the organization
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(rename_all = "camelCase")]
pub struct OrganizationUpdate {
    /// Uniue system identifier for the organization
    pub id: String,

    /// The name of of the organization
    pub name: String,

    /// Is the organization activate
    pub active: bool,
}
