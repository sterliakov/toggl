use chrono::{DateTime, Duration, Local};
use log::error;
use serde::{Deserialize, Serialize};

use crate::customization::Customization;
use crate::entities::{
    ExtendedMe, Project, ProjectId, Tag, Workspace, WorkspaceId,
};
use crate::time_entry::TimeEntry;
use crate::utils::{Client, NetResult};

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
    #[serde(default)]
    pub tags: Vec<Tag>,
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

#[derive(Clone, Debug)]
pub enum EntryEditAction {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug)]
pub struct EntryEditInfo {
    pub entry: TimeEntry,
    pub action: EntryEditAction,
}

impl State {
    pub fn update_from_context(self, me: ExtendedMe) -> Self {
        let ws_id = me
            .default_workspace_id
            .filter(|&ws| me.workspaces.iter().any(|w| w.id == ws))
            .or_else(|| me.workspaces.first().map(|ws| ws.id));
        let project_id = self
            .default_project
            .filter(|&proj| me.projects.iter().any(|p| p.id == proj));
        let earliest_entry_time = me.time_entries.iter().map(|e| e.start).min();
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
            tags: me.tags,
            default_workspace: ws_id,
            default_project: project_id,
            earliest_entry_time,
            customization: self.customization.update_from_preferences(
                me.preferences.with_beginning_of_week(me.beginning_of_week),
            ),
            ..self
        }
    }

    pub fn has_whole_last_week(&self) -> bool {
        if !self.has_more_entries {
            return true;
        }
        match self.earliest_entry_time {
            Some(time) => {
                time < self.customization.to_start_of_week(Local::now())
            }
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

    pub fn week_total(&self) -> Duration {
        let mon = self.customization.to_start_of_week(Local::now());
        let old = self
            .time_entries
            .iter()
            .filter_map(|e| {
                if e.start >= mon {
                    Some(e.get_duration())
                } else {
                    None
                }
            })
            .sum();
        match &self.running_entry {
            None => old,
            Some(e) => old + e.get_duration(),
        }
    }

    fn sort_entries(&mut self) {
        self.time_entries
            .sort_by_key(|e| std::cmp::Reverse(e.start));
    }

    pub fn apply_change(&mut self, change: EntryEditInfo) -> Result<(), ()> {
        //! Apply an optimistic update.
        //!
        //! If Ok(), the changes are unambiguous. Otherwise a full resync
        //! should be performed.

        match change.action {
            EntryEditAction::Create => {
                if self.running_entry.is_some() && change.entry.stop.is_none() {
                    // Created a running entry without explicitly stopping
                    // the previous one
                    return Err(());
                }
                if change.entry.stop.is_none() {
                    self.running_entry = Some(change.entry.clone());
                } else {
                    self.time_entries.insert(0, change.entry.clone());
                    self.sort_entries();
                }
                Ok(())
            }
            EntryEditAction::Delete => {
                if self
                    .running_entry
                    .as_ref()
                    .is_some_and(|e| e.id == change.entry.id)
                {
                    self.running_entry = None;
                }
                self.time_entries.retain(|e| e.id != change.entry.id);
                Ok(())
            }
            EntryEditAction::Update => {
                match (&self.running_entry, change.entry.stop) {
                    (None, None) => {
                        // Some entry edited to become running
                        self.running_entry = Some(change.entry.clone());
                        self.time_entries.retain(|e| e.id != change.entry.id);
                        return Ok(());
                    }
                    (Some(old), None) => {
                        if old.id == change.entry.id {
                            // Current running entry edited.
                            self.running_entry = Some(change.entry.clone());
                            return Ok(());
                        } else {
                            // Edited to make an entry running while another entry
                            // was running. Back to the server - previous entry
                            // was stopped when a new one was submitted, but we
                            // don't know exact time.
                            return Err(());
                        }
                    }
                    (Some(old), Some(_)) if old.id == change.entry.id => {
                        // Entry stopped
                        self.running_entry = None;
                        self.time_entries.insert(0, change.entry.clone());
                        self.sort_entries();
                        return Ok(());
                    }
                    _ => {}
                }
                // In all other cases the edit should belong to some old entry
                for e in self.time_entries.iter_mut() {
                    if e.id == change.entry.id {
                        change.entry.clone_into(e);
                        self.sort_entries();
                        return Ok(());
                    }
                }
                // Something went wrong - no match
                error!("Unable to update entry {}, reloading", change.entry.id);
                Err(())
            }
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

        let mut file =
            async_std::fs::File::open(Self::path()).await.map_err(|e| {
                error!("Failed to open a state file: {e}");
                StatePersistenceError::FileSystem
            })?;

        file.read_to_string(&mut contents).await.map_err(|e| {
            error!("Failed to read a state file: {e}");
            StatePersistenceError::FileSystem
        })?;

        serde_json::from_str(&contents).map_err(|e| {
            error!("Failed to parse the state file: {e}");
            StatePersistenceError::Format
        })
    }

    pub async fn save(self) -> Result<(), StatePersistenceError> {
        // This takes ownership for easier async saving
        use async_std::prelude::*;

        let json = serde_json::to_string_pretty(&self).map_err(|e| {
            error!("Failed to serialize state: {e}");
            StatePersistenceError::Format
        })?;

        let path = Self::path();

        if let Some(dir) = path.parent() {
            async_std::fs::create_dir_all(dir).await.map_err(|e| {
                error!("Failed to create parent directories: {e}");
                StatePersistenceError::FileSystem
            })?;
        }

        {
            let mut file =
                async_std::fs::File::create(path).await.map_err(|e| {
                    error!("Failed to create a state file: {e}");
                    StatePersistenceError::FileSystem
                })?;

            file.write_all(json.as_bytes()).await.map_err(|e| {
                error!("Failed to write state to the file: {e}");
                StatePersistenceError::FileSystem
            })?;
        }

        Ok(())
    }

    pub async fn delete_file(self) -> Result<(), StatePersistenceError> {
        let path = Self::path();
        async_std::fs::remove_file(path).await.map_err(|e| {
            error!("Failed to remove state file: {e}");
            StatePersistenceError::FileSystem
        })
    }

    pub async fn save_customization(&self) -> NetResult<()> {
        let client = Client::from_api_token(&self.api_token);
        self.customization
            .clone()
            .save(self.default_workspace, &client)
            .await
    }
}

#[cfg(test)]
mod test {
    use chrono::{Duration, Local, TimeDelta};

    use super::State;
    use crate::customization::WeekDay;
    use crate::entities::{Preferences, Workspace, WorkspaceId};
    use crate::test::test_client;
    use crate::time_entry::TimeEntry;
    use crate::ExtendedMe;

    #[test]
    fn test_state_load() {
        let ws = Workspace::default();
        let now = Local::now();
        let e_running = TimeEntry {
            start: now,
            duration: -1,
            ..TimeEntry::default()
        };
        let e_stopped = TimeEntry {
            start: now - TimeDelta::minutes(11),
            stop: Some(now - TimeDelta::minutes(1)),
            duration: 10 * 60,
            ..TimeEntry::default()
        };
        let e_foreign = TimeEntry {
            start: now - TimeDelta::minutes(22),
            stop: Some(now - TimeDelta::minutes(12)),
            duration: 10 * 60,
            workspace_id: WorkspaceId::new(1),
            ..TimeEntry::default()
        };
        let me = ExtendedMe {
            projects: vec![],
            workspaces: vec![ws.clone()],
            tags: vec![],
            time_entries: vec![e_running.clone(), e_stopped, e_foreign.clone()],
            beginning_of_week: 0,
            default_workspace_id: Some(WorkspaceId::new(1)),
            preferences: Preferences::default(),
        };
        let mut state = State::default().update_from_context(me);
        assert_eq!(state.running_entry, Some(e_running));
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.default_workspace, Some(ws.id));
        assert_eq!(state.default_project, None);
        assert_eq!(state.earliest_entry_time, Some(e_foreign.start));
        let running_time = Local::now() - now;
        assert!(state.week_total() > Duration::minutes(10) + running_time);
        assert!(
            state.week_total()
                < Duration::minutes(10)
                    + running_time
                    + Duration::milliseconds(200)
        );

        assert!(state.has_more_entries);
        assert!(!state.has_whole_last_week());
        state.add_entries(Vec::new().into_iter());
        assert!(!state.has_more_entries);
        assert!(state.has_whole_last_week());
    }

    #[test]
    fn test_state_load_old_enough() {
        let ws = Workspace::default();
        let now = Local::now() - TimeDelta::days(7);
        let e = TimeEntry {
            start: now - TimeDelta::minutes(11),
            stop: Some(now - TimeDelta::minutes(1)),
            duration: 10 * 60,
            ..TimeEntry::default()
        };
        let me = ExtendedMe {
            projects: vec![],
            workspaces: vec![ws.clone()],
            tags: vec![],
            time_entries: vec![e.clone()],
            beginning_of_week: 0,
            default_workspace_id: Some(WorkspaceId::new(1)),
            preferences: Preferences::default(),
        };
        let state = State::default().update_from_context(me);
        assert_eq!(state.running_entry, None);
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.earliest_entry_time, Some(e.start));
        assert_eq!(state.week_total(), Duration::zero());
        assert!(state.has_more_entries);
        assert!(state.has_whole_last_week());
    }

    #[test]
    fn test_state_preferences() {
        let me = ExtendedMe {
            projects: vec![],
            workspaces: vec![],
            tags: vec![],
            time_entries: vec![],
            beginning_of_week: 2, // Tue
            default_workspace_id: Some(WorkspaceId::new(1)),
            preferences: Preferences::default(),
        };
        let state = State::default().update_from_context(me);
        assert_eq!(
            state.customization.week_start_day,
            WeekDay(chrono::Weekday::Tue)
        );
    }

    #[test]
    fn test_state_workspace_default() {
        let ws1 = Workspace {
            id: WorkspaceId::new(10),
            ..Workspace::default()
        };
        let ws2 = Workspace {
            id: WorkspaceId::new(11),
            ..Workspace::default()
        };
        let mut me = ExtendedMe {
            projects: vec![],
            workspaces: vec![ws1.clone(), ws2.clone()],
            tags: vec![],
            time_entries: vec![],
            beginning_of_week: 2, // Tue
            default_workspace_id: Some(ws2.id),
            preferences: Preferences::default(),
        };

        let state = State::default().update_from_context(me.clone());
        assert_eq!(state.default_workspace, Some(ws2.id));

        me.default_workspace_id = None;
        let state = State::default().update_from_context(me.clone());
        assert_eq!(state.default_workspace, Some(ws1.id));

        me.default_workspace_id = Some(WorkspaceId::new(0)); // Not found
        let state = State::default().update_from_context(me);
        assert_eq!(state.default_workspace, Some(ws1.id));
    }

    #[async_std::test]
    async fn test_crud() {
        let client = test_client();
        let me = ExtendedMe::load(&client).await.expect("get me");
        let state = State::default().update_from_context(me);
        assert!(state.default_workspace.is_some());

        let mut customization = state.customization;
        let new_day = WeekDay((*customization.week_start_day).succ());
        customization.week_start_day = new_day;
        customization
            .save(state.default_workspace, &client)
            .await
            .expect("save customization");

        // Respect API limits
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        let me = ExtendedMe::load(&client).await.expect("get me");
        let state = State::default().update_from_context(me);
        assert_eq!(state.customization.week_start_day, new_day);
    }
}

#[cfg(test)]
mod test_updates {
    use chrono::{DateTime, Local, TimeDelta};

    use super::{EntryEditAction, EntryEditInfo, State};
    use crate::time_entry::TimeEntry;

    fn entry(
        now: DateTime<Local>,
        start: i64,
        duration: Option<i64>,
        id: u64,
    ) -> TimeEntry {
        if let Some(duration) = duration {
            TimeEntry {
                start: now - TimeDelta::minutes(start),
                stop: Some(now - TimeDelta::minutes(start - duration)),
                duration: duration * 60,
                id,
                ..TimeEntry::default()
            }
        } else {
            TimeEntry {
                start: now - TimeDelta::minutes(start),
                stop: None,
                duration: -1,
                id,
                ..TimeEntry::default()
            }
        }
    }

    #[test]
    fn test_state_optimistic_update_completed() {
        let now = Local::now() - TimeDelta::days(1);
        // Update old entries
        let running = entry(now, 1, None, 2);
        for running_entry in [None, Some(running)] {
            let mut state = State {
                time_entries: vec![entry(now, 11, Some(10), 1)],
                running_entry: running_entry.clone(),
                ..State::default()
            };

            assert!(state
                .apply_change(EntryEditInfo {
                    action: EntryEditAction::Update,
                    entry: entry(now, 21, Some(10), 1),
                })
                .is_ok());
            assert_eq!(state.time_entries.len(), 1);
            assert_eq!(state.time_entries[0], entry(now, 21, Some(10), 1));
            assert_eq!(state.running_entry, running_entry);

            assert!(state
                .apply_change(EntryEditInfo {
                    action: EntryEditAction::Delete,
                    entry: entry(now, 21, Some(10), 1),
                })
                .is_ok());
            assert_eq!(state.running_entry, running_entry);
            assert_eq!(state.time_entries.len(), 0);

            assert!(state
                .apply_change(EntryEditInfo {
                    action: EntryEditAction::Create,
                    entry: entry(now, 21, Some(10), 1),
                })
                .is_ok());
            assert_eq!(state.time_entries.len(), 1);
            assert_eq!(state.time_entries[0], entry(now, 21, Some(10), 1));
            assert_eq!(state.running_entry, running_entry);

            // Make it running again, conflicts if already running
            let res = state.apply_change(EntryEditInfo {
                action: EntryEditAction::Update,
                entry: entry(now, 21, None, 1),
            });
            if running_entry.is_some() {
                assert!(res.is_err());
                // state unmodified
            } else {
                assert!(state.time_entries.is_empty());
                assert_eq!(state.running_entry, Some(entry(now, 21, None, 1)));
            }
        }
    }

    #[test]
    fn test_state_optimistic_update_running() {
        let now = Local::now() - TimeDelta::days(1);
        let old = entry(now, 21, Some(10), 1);
        let mut state = State {
            time_entries: vec![old.clone()],
            running_entry: Some(entry(now, 11, None, 2)),
            ..State::default()
        };

        assert!(state
            .apply_change(EntryEditInfo {
                action: EntryEditAction::Update,
                entry: entry(now, 12, None, 2),
            })
            .is_ok());
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.time_entries[0], old);
        assert_eq!(state.running_entry, Some(entry(now, 12, None, 2)));

        assert!(state
            .apply_change(EntryEditInfo {
                action: EntryEditAction::Update,
                entry: entry(now, 11, Some(1), 2),
            })
            .is_ok());
        assert_eq!(state.time_entries.len(), 2);
        assert_eq!(state.time_entries, [entry(now, 11, Some(1), 2), old]);
        assert_eq!(state.running_entry, None);
    }

    #[test]
    fn test_state_optimistic_update_running_2() {
        let now = Local::now() - TimeDelta::days(1);
        let old = entry(now, 21, Some(10), 1);
        let mut state = State {
            time_entries: vec![old.clone()],
            running_entry: Some(entry(now, 11, None, 2)),
            ..State::default()
        };

        assert!(state
            .apply_change(EntryEditInfo {
                action: EntryEditAction::Delete,
                entry: entry(now, 12, None, 2),
            })
            .is_ok());
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.time_entries[0], old);
        assert_eq!(state.running_entry, None);

        assert!(state
            .apply_change(EntryEditInfo {
                action: EntryEditAction::Create,
                entry: entry(now, 11, None, 2),
            })
            .is_ok());
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.time_entries[0], old);
        assert_eq!(state.running_entry, Some(entry(now, 11, None, 2)));

        assert!(state
            .apply_change(EntryEditInfo {
                action: EntryEditAction::Create,
                entry: entry(now, 11, None, 3),
            })
            .is_err());
    }
}
