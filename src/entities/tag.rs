use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tag {
    pub id: u64,
    pub name: String,
}
