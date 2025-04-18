use log::info;
use serde::{Deserialize, Serialize};

use crate::entities::{Preferences, Project, Workspace};
use crate::time_entry::TimeEntry;
use crate::utils::{Client, NetResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedMe {
    pub api_token: String,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub time_entries: Vec<TimeEntry>,
    #[serde(skip)]
    pub preferences: Preferences,
}

impl ExtendedMe {
    pub async fn load(client: &Client) -> NetResult<Self> {
        info!("Fetching profile and related objects...");
        let mut rsp = client
            .get(format!(
                "{}/api/v9/me?with_related_data=true",
                Client::BASE_URL
            ))
            .send()
            .await?;
        Client::check_status(&mut rsp).await?;
        rsp.body_json().await
    }
}
