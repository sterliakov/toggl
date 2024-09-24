use iced::Element;
use iced::widget::text;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::client::{Client, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeEntry {
    pub at: String,
    pub billable: bool,
    // client_name: String;
    pub description: Option<String>,
    pub duration: i64,
    // "duronly": {
    //     "description": "Used to create a TE with a duration but without a stop time, this field is deprecated for GET endpoints where the value will always be true.",
    //     "type": "boolean"
    // },
    pub id: u64,
    pub permissions: Option<Vec<String>>,
    // "pid": {
    //     "description": "Project ID, legacy field",
    //     "type": "integer"
    // },
    // pub project_active: bool,
    // pub project_billable: bool,
    // pub project_color: String,
    pub project_id: Option<u64>,
    // pub project_name: String,
    // "shared_with": {
    //     "description": "Indicates who the time entry has been shared with",
    //     "type": "array",
    //     "items": {
    //         "$ref": "#/definitions/models.TimeEntrySharedWith"
    //     }
    // },
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub stop: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub server_deleted_at: Option<OffsetDateTime>,
    pub tag_ids: Vec<u64>,
    pub tags: Vec<String>,
    pub task_id: Option<u64>,
    // pub task_name: String,
    // "tid": {
    //     "description": "Task ID, legacy field",
    //     "type": "integer"
    // },
    // "uid": {
    //     "description": "Time Entry creator ID, legacy field",
    //     "type": "integer"
    // },
    // pub user_avatar_url: String,
    pub user_id: u64,
    // pub user_name: String,
    // "wid": {
    //     "description": "Workspace ID, legacy field",
    //     "type": "integer"
    // },
    pub workspace_id: u64,
}

impl TimeEntry {
    pub async fn load(client: &Client) -> Result<Vec<Self>> {
        client
            .get([Client::BASE_URL, "/api/v9/me/time_entries"].join(""))
            .send()
            .await?
            .body_json::<Vec<Self>>()
            .await
    }
}

impl TimeEntry {
    pub fn view(&self) -> Element<()> {
        text(
            self.description
                .clone()
                .unwrap_or("<NO DESCRIPTION>".to_string()),
        )
        .into()
    }
}

#[cfg(test)]
mod test {
    use super::TimeEntry;
    use crate::client::Client;

    #[async_std::test]
    async fn test_load() {
        let client = Client::from_email_password(
            &std::env::var("TEST_EMAIL").expect("Please pass TEST_EMAIL"),
            &std::env::var("TEST_PASSWORD").expect("Please pass TEST_PASSWORD"),
        );
        let entries = TimeEntry::load(&client).await.expect("Failed");
        assert_ne!(entries.len(), 0);
    }
}
