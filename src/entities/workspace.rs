use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct WorkspaceId(u64);

impl WorkspaceId {
    #[cfg(test)]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
}
