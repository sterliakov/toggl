use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub active: bool,
    pub color: String,
}

impl std::fmt::Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaybeProject {
    Some(Project),
    None,
}

impl std::fmt::Display for MaybeProject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaybeProject::Some(p) => p.fmt(f),
            MaybeProject::None => f.write_str("---"),
        }
    }
}

impl From<Project> for MaybeProject {
    fn from(value: Project) -> Self {
        Self::Some(value)
    }
}

impl From<Option<Project>> for MaybeProject {
    fn from(value: Option<Project>) -> Self {
        match value {
            Some(p) => Self::Some(p),
            None => Self::None,
        }
    }
}

impl From<MaybeProject> for Option<Project> {
    fn from(val: MaybeProject) -> Self {
        match val {
            MaybeProject::Some(p) => Some(p),
            MaybeProject::None => None,
        }
    }
}
