use core::str;
use std::time::Duration;

use iced::alignment::Vertical;
use iced::widget::{button, row, text};
use iced::{Element, Length};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::client::{Client, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeEntry {
    pub at: String,
    pub billable: bool,
    pub description: Option<String>,
    pub duration: i64,
    pub id: u64,
    pub permissions: Option<Vec<String>>,
    pub project_id: Option<u64>,
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub stop: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub server_deleted_at: Option<OffsetDateTime>,
    pub tag_ids: Vec<u64>,
    pub tags: Vec<String>,
    pub task_id: Option<u64>,
    pub user_id: u64,
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

    pub async fn save(&self, client: &Client) -> Result<()> {
        let mut res = client
            .put(
                [
                    Client::BASE_URL.to_string(),
                    format!(
                        "/api/v9/workspaces/{}/time_entries/{}",
                        self.workspace_id, self.id
                    ),
                ]
                .join(""),
            )
            .body_json(&self)?
            .send()
            .await?;
        let status = res.status();
        if !status.is_success() {
            Err(surf::Error::from_str(
                status,
                str::from_utf8(&res.body_bytes().await?)?.to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub async fn delete(self, client: &Client) -> Result<()> {
        let mut res = client
            .delete(
                [
                    Client::BASE_URL.to_string(),
                    format!(
                        "/api/v9/workspaces/{}/time_entries/{}",
                        self.workspace_id, self.id
                    ),
                ]
                .join(""),
            )
            .send()
            .await?;
        let status = res.status();
        if !status.is_success() {
            Err(surf::Error::from_str(
                status,
                str::from_utf8(&res.body_bytes().await?)?.to_string(),
            ))
        } else {
            Ok(())
        }
    }

    fn duration_string(&self) -> String {
        duration_to_hms(&Duration::from_secs(self.duration.max(0) as u64))
    }
}

#[derive(Clone, Debug)]
pub enum TimeEntryMessage {
    Edit(usize),
}

impl TimeEntry {
    pub fn view(&self, i: usize) -> Element<TimeEntryMessage> {
        let name = self.description.clone().unwrap_or("<>".to_string());
        button(
            row![
                text(name).width(Length::Fill),
                text(self.duration_string()).width(Length::Fixed(60f32))
            ]
            .spacing(10)
            .align_y(Vertical::Center),
        )
        .on_press(TimeEntryMessage::Edit(i))
        .clip(true)
        .style(button::text)
        .into()
    }
}

fn duration_to_hms(duration: &Duration) -> String {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let hours = (duration.as_secs() / 60) / 60;
    format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
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
