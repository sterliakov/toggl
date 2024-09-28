use crate::{
    client::{Client, Result as NetResult},
    project::Project,
    time_entry::TimeEntry,
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedMe {
    pub api_token: String,
    pub projects: Vec<Project>,
    pub workspaces: Vec<Workspace>,
    pub time_entries: Vec<TimeEntry>,
}

impl ExtendedMe {
    pub async fn load(client: &Client) -> NetResult<Self> {
        client
            .get(
                [Client::BASE_URL, "/api/v9/me?with_related_data=true"]
                    .join(""),
            )
            .send()
            .await?
            .body_json()
            .await
    }
}