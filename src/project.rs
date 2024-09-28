use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub active: bool,
    pub color: String,
}
