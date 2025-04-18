use log::info;
use serde::{Deserialize, Serialize};

use crate::utils::{Client, NetResult};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub date_format: String,
    #[serde(rename = "timeofday_format")]
    pub time_format: String,
    #[serde(default, rename = "beginningOfWeek")]
    pub beginning_of_week: u8,
}

impl Preferences {
    pub async fn load(client: &Client) -> NetResult<Self> {
        info!("Fetching preferences...");
        let mut rsp = client
            .get(format!("{}/api/v9/me/preferences", Client::BASE_URL))
            .send()
            .await?;
        Client::check_status(&mut rsp).await?;
        rsp.body_json().await
    }

    pub async fn save(&self, client: &Client) -> NetResult<()> {
        info!("Saving preferences...");
        let mut rsp = client
            .post(format!("{}/api/v9/me/preferences", Client::BASE_URL))
            .body_json(&self)
            .expect("serialize Preferences")
            .send()
            .await?;
        Client::check_status(&mut rsp).await
    }
}
