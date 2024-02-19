use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
