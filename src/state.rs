use chrono::{DateTime, Local, TimeDelta};
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
    /// Earliest entry time (may not be in `time_entries` if comes from other workspace)
    pub earliest_entry_time: Option<DateTime<Local>>,
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
        let earliest_entry_time = me.time_entries.last().map(|last| last.start);
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
            // If we got 0 entries, no reason to load more.
            has_more_entries: earliest_entry_time.is_some(),
            projects: me.projects,
            workspaces: me.workspaces,
            default_workspace: ws_id,
            default_project: project_id,
            earliest_entry_time,
            ..self
        }
    }

    pub fn has_whole_last_week(&self) -> bool {
        if !self.has_more_entries {
            return true;
        }
        match self.earliest_entry_time {
            // Yes, just load 7 days. Yes, we will load more than necessary.
            // Yes, it's easier than trying to find Monday 00:00:00 of this week
            // and it won't harm if we load once again.
            Some(time) => time <= Local::now() - TimeDelta::days(7),
            None => true,
        }
    }

    pub fn add_entries(&mut self, entries: impl Iterator<Item = TimeEntry>) {
        let mut earliest = None;
        self.time_entries.extend(
            entries
                .inspect(|e| {
                    earliest = match earliest {
                        None => Some(e.start),
                        Some(v) => Some(v.min(e.start)),
                    }
                })
                .filter(|e| Some(e.workspace_id) == self.default_workspace),
        );
        self.has_more_entries = earliest.is_some();
        self.earliest_entry_time = earliest.or(self.earliest_entry_time);
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
