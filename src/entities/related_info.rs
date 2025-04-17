use log::debug;
use serde::{Deserialize, Serialize};

use super::{Project, Workspace};
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
    #[cfg(not(test))]
    beginning_of_week: u8,
    #[cfg(test)]
    pub beginning_of_week: u8,
}

impl ExtendedMe {
    pub fn first_week_day(&self) -> chrono::Weekday {
        // Toggl thinks Sun = 0, chrono thinks Mon = 0
        let off_by_one: chrono::Weekday =
            self.beginning_of_week.try_into().expect("bad start day");
        off_by_one.pred()
    }

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
