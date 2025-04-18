use log::info;
use serde::{Deserialize, Serialize};

use super::WorkspaceId;
use crate::utils::{Client, NetResult};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub date_format: String,
    #[serde(rename = "timeofday_format")]
    pub time_format: String,
    /// There is a similar field in prefs, but it's always Sunday.
    ///
    /// The actual value comes from `/me` and is set externally.
    #[serde(skip)]
    pub beginning_of_week: u8,
}

impl Preferences {
    pub fn with_beginning_of_week(self, day: u8) -> Self {
        Self {
            beginning_of_week: day,
            ..self
        }
    }
    pub async fn load(client: &Client) -> NetResult<Self> {
        info!("Fetching preferences...");
        let mut rsp = client
            .get(format!("{}/api/v9/me/preferences", Client::BASE_URL))
            .send()
            .await?;
        Client::check_status(&mut rsp).await?;
        rsp.body_json().await
    }

    pub async fn save(
        &self,
        default_workspace_id: Option<WorkspaceId>,
        client: &Client,
    ) -> NetResult<()> {
        futures::try_join!(
            self.save_base(client),
            self.save_profile(default_workspace_id, client)
        )?;
        Ok(())
    }

    async fn save_base(&self, client: &Client) -> NetResult<()> {
        info!("Saving main preferences...");
        let mut rsp = client
            .post(format!("{}/api/v9/me/preferences", Client::BASE_URL))
            .body_json(&self)
            .expect("serialize Preferences")
            .send()
            .await?;
        Client::check_status(&mut rsp).await
    }

    async fn save_profile(
        &self,
        default_workspace_id: Option<WorkspaceId>,
        client: &Client,
    ) -> NetResult<()> {
        info!("Saving beginning of week...");
        let mut rsp = client
            .put(format!("{}/api/v9/me", Client::BASE_URL))
            .body_json(&ProfilePart {
                beginning_of_week: self.beginning_of_week,
                default_workspace_id,
            })
            .expect("serialize ProfilePart")
            .send()
            .await?;
        Client::check_status(&mut rsp).await
    }
}

#[derive(Serialize)]
struct ProfilePart {
    beginning_of_week: u8,
    default_workspace_id: Option<WorkspaceId>,
}
