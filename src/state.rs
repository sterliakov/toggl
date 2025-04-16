use serde::{Deserialize, Serialize};

use crate::customization::Customization;
use crate::entities::{ExtendedMe, Project, ProjectId, Workspace, WorkspaceId};
use crate::time_entry::TimeEntry;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub api_token: String,
    pub time_entries: Vec<TimeEntry>,
    pub running_entry: Option<TimeEntry>,
    pub has_more_entries: bool,
    pub projects: Vec<Project>,
    pub workspaces: Vec<Workspace>,
    pub default_workspace: Option<WorkspaceId>,
    pub default_project: Option<ProjectId>,
    pub customization: Customization,
}

#[derive(Debug, Clone)]
pub enum StatePersistenceError {
    FileSystem,
    Format,
}

impl std::fmt::Display for StatePersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::FileSystem => "Failed to read/write a state file.",
            Self::Format => "State file format not recognized.",
        };
        msg.fmt(f)
    }
}

impl State {
    pub fn update_from_context(self, me: ExtendedMe) -> Self {
        let ws_id = self
            .default_workspace
            .filter(|&ws| me.workspaces.iter().any(|w| w.id == ws))
            .or_else(|| me.workspaces.first().map(|ws| ws.id));
        let project_id = self
            .default_project
            .filter(|&proj| me.projects.iter().any(|p| p.id == proj));
        let (running_entry, time_entries) =
            TimeEntry::split_running(if let Some(ws_id) = ws_id {
                me.time_entries
                    .into_iter()
                    .filter(|e| e.workspace_id == ws_id)
                    .collect()
            } else {
                me.time_entries
            });
        Self {
            running_entry,
            time_entries,
            has_more_entries: true,
            projects: me.projects,
            workspaces: me.workspaces,
            default_workspace: ws_id,
            default_project: project_id,
            ..self
        }
    }
}

impl State {
    fn path() -> std::path::PathBuf {
        let mut path = if let Some(project_dirs) =
            directories_next::ProjectDirs::from("rs", "Iced", "toggl-tracker")
        {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        };

        path.push("toggl.json");
        path
    }

    pub async fn load() -> Result<Box<Self>, StatePersistenceError> {
        use async_std::prelude::*;

        let mut contents = String::new();

        let mut file = async_std::fs::File::open(Self::path())
            .await
            .map_err(|_| StatePersistenceError::FileSystem)?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|_| StatePersistenceError::FileSystem)?;

        serde_json::from_str(&contents)
            .map_err(|_| StatePersistenceError::Format)
    }

    pub async fn save(self) -> Result<(), StatePersistenceError> {
        // This takes ownership for easier async saving
        use async_std::prelude::*;

        let json = serde_json::to_string_pretty(&self)
            .map_err(|_| StatePersistenceError::Format)?;

        let path = Self::path();

        if let Some(dir) = path.parent() {
            async_std::fs::create_dir_all(dir)
                .await
                .map_err(|_| StatePersistenceError::FileSystem)?;
        }

        {
            let mut file = async_std::fs::File::create(path)
                .await
                .map_err(|_| StatePersistenceError::FileSystem)?;

            file.write_all(json.as_bytes())
                .await
                .map_err(|_| StatePersistenceError::FileSystem)?;
        }

        Ok(())
    }
}
