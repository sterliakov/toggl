use log::info;
use serde::Deserialize;

use super::{Tag, WorkspaceId};
use crate::entities::{Preferences, Project, Workspace};
use crate::time_entry::TimeEntry;
use crate::utils::{maybe_vec_deserialize, Client, NetResult};

#[derive(Clone, Debug, Deserialize)]
pub struct ExtendedMe {
    #[serde(default, deserialize_with = "maybe_vec_deserialize")]
    pub projects: Vec<Project>,
    #[serde(default, deserialize_with = "maybe_vec_deserialize")]
    pub workspaces: Vec<Workspace>,
    #[serde(default, deserialize_with = "maybe_vec_deserialize")]
    pub time_entries: Vec<TimeEntry>,
    #[serde(default, deserialize_with = "maybe_vec_deserialize")]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub beginning_of_week: u8,
    #[serde(default)]
    pub default_workspace_id: Option<WorkspaceId>,
    #[serde(skip)]
    pub preferences: Preferences,
}

impl ExtendedMe {
    pub async fn load(client: &Client) -> NetResult<Self> {
        info!("Fetching profile and related objects...");
        client
            .get(format!(
                "{}/api/v9/me?with_related_data=true",
                Client::BASE_URL
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }
}
