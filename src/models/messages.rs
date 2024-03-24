use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GenericMessage {
    pub detail: String,
}
