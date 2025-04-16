use log::debug;
use serde::{Deserialize, Serialize};

use crate::project::Project;
use crate::time_entry::TimeEntry;
use crate::utils::{Client, NetResult};
use crate::workspace::Workspace;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedMe {
    pub api_token: String,
    pub projects: Vec<Project>,
    pub workspaces: Vec<Workspace>,
    pub time_entries: Vec<TimeEntry>,
}

impl ExtendedMe {
    pub async fn load(client: &Client) -> NetResult<Self> {
        debug!("Fetching profile and related objects...");
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
