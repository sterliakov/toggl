use std::collections::HashMap;

use chrono::{DateTime, Duration, Local};
use itertools::Itertools as _;
use log::error;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use crate::customization::Customization;
use crate::entities::{
    ExtendedMe, Project, ProjectId, Tag, Workspace, WorkspaceId,
};
use crate::time_entry::TimeEntry;
use crate::utils::{Client, NetResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub active_profile: String,
    pub profiles: HashMap<String, Profile>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_profile: "default".to_owned(),
            profiles: {
                let mut dict = HashMap::new();
                dict.insert("default".to_owned(), Profile::default());
                dict
            },
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    api_token: String,
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
    customization: Customization,
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

impl Profile {
    pub fn new(api_token: String) -> Self {
        Self {
            api_token,
            ..Self::default()
        }
    }

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
                &me.preferences.with_beginning_of_week(me.beginning_of_week),
            ),
            ..self
        }
    }

    pub fn has_whole_last_week(&self) -> bool {
        if !self.has_more_entries {
            return true;
        }
        self.earliest_entry_time.is_none_or(|time| {
            time < self.customization.to_start_of_week(Local::now())
        })
    }

    pub fn add_entries(&mut self, entries: impl Iterator<Item = TimeEntry>) {
        let mut earliest: Option<DateTime<Local>> = None;
        self.time_entries.extend(
            entries
                .inspect(|e| {
                    earliest = earliest
                        .map_or(Some(e.start), |v| Some(v.min(e.start)));
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
            .filter(|&e| e.start >= mon)
            .map(super::time_entry::TimeEntry::get_duration)
            .sum();
        self.running_entry
            .as_ref()
            .map_or(old, |e| old + e.get_duration())
    }

    fn sort_entries(&mut self) {
        self.time_entries
            .sort_by_key(|e| std::cmp::Reverse(e.start));
    }

    pub fn apply_change(&mut self, change: &EntryEditInfo) -> Result<(), ()> {
        //! Apply an optimistic update.
        //!
        //! If `Ok()`, the changes are unambiguous. Otherwise a full resync
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
            #[expect(clippy::pattern_type_mismatch)]
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
                        }
                        // Edited to make an entry running while another entry
                        // was running. Back to the server - previous entry
                        // was stopped when a new one was submitted, but we
                        // don't know exact time.
                        return Err(());
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
                for e in &mut self.time_entries {
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

    pub async fn save_customization(&self) -> NetResult<()> {
        let client = Client::from_api_token(&self.api_token);
        self.customization
            .clone()
            .save(self.default_workspace, &client)
            .await
    }
}

impl State {
    pub fn ensure_profile(&mut self, email: String, api_token: String) {
        self.profiles
            .entry(email)
            .and_modify(|p| p.api_token.clone_from(&api_token))
            .or_insert_with(|| Profile::new(api_token));
    }
    pub fn select_profile(&mut self, email: String) {
        self.active_profile = email;
    }

    pub fn api_token(&self) -> String {
        self.current_profile().api_token.clone()
    }

    pub fn customization(&self) -> &Customization {
        &self.current_profile().customization
    }

    pub fn customization_mut(&mut self) -> &mut Customization {
        &mut self.current_profile_mut().customization
    }

    fn path() -> std::path::PathBuf {
        directories_next::ProjectDirs::from("rs", "Iced", "toggl-tracker")
            .map_or_else(
                || std::env::current_dir().unwrap_or_default(),
                |project_dirs| project_dirs.data_dir().into(),
            )
            .join("toggl.json")
    }

    pub async fn load() -> Result<Box<Self>, StatePersistenceError> {
        let mut contents = String::new();

        let mut file =
            tokio::fs::File::open(Self::path()).await.map_err(|e| {
                error!("Failed to open a state file: {e}");
                StatePersistenceError::FileSystem
            })?;

        file.read_to_string(&mut contents).await.map_err(|e| {
            error!("Failed to read a state file: {e}");
            StatePersistenceError::FileSystem
        })?;

        let mut state: Self = serde_json::from_str(&contents).map_err(|e| {
            error!("Failed to parse the state file: {e}");
            StatePersistenceError::Format
        })?;
        state.profiles.retain(|_, p| !p.api_token.is_empty());
        if state.profiles.is_empty() {
            Err(StatePersistenceError::Format)
        } else {
            Ok(state.into())
        }
    }

    pub async fn save(self) -> Result<(), StatePersistenceError> {
        // This takes ownership for easier async saving

        let json = serde_json::to_string_pretty(&self).map_err(|e| {
            error!("Failed to serialize state: {e}");
            StatePersistenceError::Format
        })?;

        let path = Self::path();

        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await.map_err(|e| {
                error!("Failed to create parent directories: {e}");
                StatePersistenceError::FileSystem
            })?;
        }

        {
            let mut file =
                tokio::fs::File::create(path).await.map_err(|e| {
                    error!("Failed to create a state file: {e}");
                    StatePersistenceError::FileSystem
                })?;

            file.write_all(json.as_bytes()).await.map_err(|e| {
                error!("Failed to write state to the file: {e}");
                StatePersistenceError::FileSystem
            })?;
        };

        Ok(())
    }

    async fn delete_file() -> Result<(), StatePersistenceError> {
        let path = Self::path();
        tokio::fs::remove_file(path).await.map_err(|e| {
            error!("Failed to remove state file: {e}");
            StatePersistenceError::FileSystem
        })
    }

    pub fn current_profile(&self) -> &Profile {
        self.profiles.get(&self.active_profile).unwrap_or_else(|| {
            panic!("Profile '{}' not found", self.active_profile)
        })
    }

    pub fn current_profile_mut(&mut self) -> &mut Profile {
        self.profiles
            .get_mut(&self.active_profile)
            .unwrap_or_else(|| {
                panic!("Profile '{}' not found", self.active_profile)
            })
    }

    pub async fn remove_profile(
        mut self,
        profile_name: &str,
    ) -> Result<Option<Self>, StatePersistenceError> {
        if self.profiles.len() > 1 {
            self.profiles.retain(|name, _| name != profile_name);
            if profile_name == self.active_profile {
                let next_name =
                    &self.profile_names().next().expect("to find profile");
                self.active_profile.clone_from(next_name);
            }
            Ok(Some(self))
        } else {
            Self::delete_file().await?;
            Ok(None)
        }
    }

    pub fn profile_names(&self) -> impl Iterator<Item = String> + '_ {
        self.profiles.keys().sorted().cloned()
    }

    pub fn update_from_context(&mut self, me: ExtendedMe) {
        let updated = self.current_profile().clone().update_from_context(me);
        self.profiles.insert(self.active_profile.clone(), updated);
    }

    pub fn apply_change(&mut self, change: &EntryEditInfo) -> Result<(), ()> {
        //! Apply an optimistic update.
        //!
        //! If `Ok()`, the changes are unambiguous. Otherwise a full resync
        //! should be performed.

        self.current_profile_mut().apply_change(change)
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::shadow_unrelated)]

    use chrono::{Duration, Local, TimeDelta};

    use super::Profile;
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
        let mut state = Profile::default().update_from_context(me);
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
            workspaces: vec![ws],
            tags: vec![],
            time_entries: vec![e.clone()],
            beginning_of_week: 0,
            default_workspace_id: Some(WorkspaceId::new(1)),
            preferences: Preferences::default(),
        };
        let state = Profile::default().update_from_context(me);
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
        let state = Profile::default().update_from_context(me);
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

        let state = Profile::default().update_from_context(me.clone());
        assert_eq!(state.default_workspace, Some(ws2.id));

        me.default_workspace_id = None;
        let state = Profile::default().update_from_context(me.clone());
        assert_eq!(state.default_workspace, Some(ws1.id));

        me.default_workspace_id = Some(WorkspaceId::new(0)); // Not found
        let state = Profile::default().update_from_context(me);
        assert_eq!(state.default_workspace, Some(ws1.id));
    }

    #[tokio::test]
    async fn test_crud() {
        let client = test_client();
        let me = ExtendedMe::load(&client).await.expect("get me");
        let state = Profile::default().update_from_context(me);
        assert!(state.default_workspace.is_some());

        let mut customization = state.customization;
        let new_day = WeekDay((*customization.week_start_day).succ());
        customization.week_start_day = new_day;
        customization
            .save(state.default_workspace, &client)
            .await
            .expect("save customization");

        // Respect API limits
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let me = ExtendedMe::load(&client).await.expect("get me");
        let state = Profile::default().update_from_context(me);
        assert_eq!(state.customization.week_start_day, new_day);
    }
}

#[cfg(test)]
mod test_updates {
    use chrono::{DateTime, Local, TimeDelta};

    use super::{EntryEditAction, EntryEditInfo, Profile};
    use crate::time_entry::TimeEntry;

    fn entry(
        now: DateTime<Local>,
        start: i64,
        duration: Option<i64>,
        id: u64,
    ) -> TimeEntry {
        duration.map_or_else(
            || TimeEntry {
                start: now - TimeDelta::minutes(start),
                stop: None,
                duration: -1,
                id,
                ..TimeEntry::default()
            },
            |duration| TimeEntry {
                start: now - TimeDelta::minutes(start),
                stop: Some(now - TimeDelta::minutes(start - duration)),
                duration: duration * 60,
                id,
                ..TimeEntry::default()
            },
        )
    }

    #[test]
    fn test_state_optimistic_update_completed() {
        let now = Local::now() - TimeDelta::days(1);
        // Update old entries
        let running = entry(now, 1, None, 2);
        for running_entry in [None, Some(running)] {
            let mut state = Profile {
                time_entries: vec![entry(now, 11, Some(10), 1)],
                running_entry: running_entry.clone(),
                ..Profile::default()
            };

            assert!(state
                .apply_change(&EntryEditInfo {
                    action: EntryEditAction::Update,
                    entry: entry(now, 21, Some(10), 1),
                })
                .is_ok());
            assert_eq!(state.time_entries.len(), 1);
            assert_eq!(state.time_entries[0], entry(now, 21, Some(10), 1));
            assert_eq!(state.running_entry, running_entry);

            assert!(state
                .apply_change(&EntryEditInfo {
                    action: EntryEditAction::Delete,
                    entry: entry(now, 21, Some(10), 1),
                })
                .is_ok());
            assert_eq!(state.running_entry, running_entry);
            assert_eq!(state.time_entries.len(), 0);

            assert!(state
                .apply_change(&EntryEditInfo {
                    action: EntryEditAction::Create,
                    entry: entry(now, 21, Some(10), 1),
                })
                .is_ok());
            assert_eq!(state.time_entries.len(), 1);
            assert_eq!(state.time_entries[0], entry(now, 21, Some(10), 1));
            assert_eq!(state.running_entry, running_entry);

            // Make it running again, conflicts if already running
            let res = state.apply_change(&EntryEditInfo {
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
        let mut state = Profile {
            time_entries: vec![old.clone()],
            running_entry: Some(entry(now, 11, None, 2)),
            ..Profile::default()
        };

        assert!(state
            .apply_change(&EntryEditInfo {
                action: EntryEditAction::Update,
                entry: entry(now, 12, None, 2),
            })
            .is_ok());
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.time_entries[0], old);
        assert_eq!(state.running_entry, Some(entry(now, 12, None, 2)));

        assert!(state
            .apply_change(&EntryEditInfo {
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
        let mut state = Profile {
            time_entries: vec![old.clone()],
            running_entry: Some(entry(now, 11, None, 2)),
            ..Profile::default()
        };

        assert!(state
            .apply_change(&EntryEditInfo {
                action: EntryEditAction::Delete,
                entry: entry(now, 12, None, 2),
            })
            .is_ok());
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.time_entries[0], old);
        assert_eq!(state.running_entry, None);

        assert!(state
            .apply_change(&EntryEditInfo {
                action: EntryEditAction::Create,
                entry: entry(now, 11, None, 2),
            })
            .is_ok());
        assert_eq!(state.time_entries.len(), 1);
        assert_eq!(state.time_entries[0], old);
        assert_eq!(state.running_entry, Some(entry(now, 11, None, 2)));

        assert!(state
            .apply_change(&EntryEditInfo {
                action: EntryEditAction::Create,
                entry: entry(now, 11, None, 3),
            })
            .is_err());
    }
}
