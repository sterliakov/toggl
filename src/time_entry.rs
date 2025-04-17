use chrono::{DateTime, Duration, Local};
use iced::alignment::Vertical;
use iced::widget::{button, column, row, text};
use iced::{Element, Length};
use log::debug;
use serde::{Deserialize, Serialize, Serializer};

use crate::entities::{MaybeProject, Project, ProjectId, WorkspaceId};
use crate::utils::{duration_to_hms, Client, NetResult};
use crate::widgets::icon_button;

fn datetime_serialize_utc<S: Serializer>(
    x: &DateTime<Local>,
    s: S,
) -> Result<S::Ok, S::Error> {
    x.to_utc().serialize(s)
}

fn maybe_datetime_serialize_utc<S: Serializer>(
    x: &Option<DateTime<Local>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    x.map(|d| d.to_utc()).serialize(s)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub at: String,
    pub billable: bool,
    pub description: Option<String>,
    pub duration: i64,
    pub id: u64,
    pub permissions: Option<Vec<String>>,
    pub project_id: Option<ProjectId>,
    #[serde(serialize_with = "datetime_serialize_utc")]
    pub start: DateTime<Local>,
    #[serde(serialize_with = "maybe_datetime_serialize_utc")]
    pub stop: Option<DateTime<Local>>,
    #[serde(serialize_with = "maybe_datetime_serialize_utc")]
    pub server_deleted_at: Option<DateTime<Local>>,
    pub tag_ids: Vec<u64>,
    pub tags: Vec<String>,
    pub task_id: Option<u64>,
    pub user_id: u64,
    pub workspace_id: WorkspaceId,
}

impl TimeEntry {
    pub async fn load(
        before: Option<DateTime<Local>>,
        client: &Client,
    ) -> NetResult<Vec<Self>> {
        #[derive(Serialize)]
        struct QueryParams {
            #[serde(serialize_with = "maybe_datetime_serialize_utc")]
            before: Option<DateTime<Local>>,
        }

        let mut res = client
            .get(format!("{}/api/v9/me/time_entries", Client::BASE_URL))
            .query(&QueryParams { before })?
            .send()
            .await?;
        Client::check_status(&mut res).await?;
        let entries = res.body_json::<Vec<Self>>().await?;
        if before.is_none() {
            Ok(entries)
        } else {
            // The API parses this bound as inclusive, we don't need duplicates
            Ok(entries[1..].to_vec())
        }
    }

    pub fn split_running(all_entries: Vec<Self>) -> (Option<Self>, Vec<Self>) {
        match &all_entries[..] {
            [] => (None, vec![]),
            [head, rest @ ..] => {
                if head.duration < 0 {
                    (Some(head.clone()), rest.to_vec())
                } else {
                    (None, all_entries)
                }
            }
        }
    }

    pub async fn save(&self, client: &Client) -> NetResult<()> {
        debug!("Updating a time entry {}...", self.id);
        let mut res = client
            .put(format!(
                "{}/api/v9/workspaces/{}/time_entries/{}",
                Client::BASE_URL,
                self.workspace_id,
                self.id
            ))
            .body_json(&self)?
            .send()
            .await?;
        Client::check_status(&mut res).await
    }

    pub async fn stop(&self, client: &Client) -> NetResult<()> {
        debug!("Stopping a time entry {}...", self.id);
        assert!(self.stop.is_none());
        let mut res = client
            .patch(format!(
                "{}/api/v9/workspaces/{}/time_entries/{}/stop",
                Client::BASE_URL,
                self.workspace_id,
                self.id
            ))
            .send()
            .await?;
        Client::check_status(&mut res).await
    }

    pub async fn delete(self, client: &Client) -> NetResult<()> {
        debug!("Deleting a time entry {}...", self.id);
        let mut res = client
            .delete(format!(
                "{}/api/v9/workspaces/{}/time_entries/{}",
                Client::BASE_URL,
                self.workspace_id,
                self.id
            ))
            .send()
            .await?;
        Client::check_status(&mut res).await
    }

    pub async fn create_running(
        description: Option<String>,
        workspace_id: WorkspaceId,
        project_id: Option<ProjectId>,
        client: &Client,
    ) -> NetResult<()> {
        debug!("Creating a time entry...");
        let entry = CreateTimeEntry {
            created_with: "ST-Toggl-Client".to_string(),
            description,
            duration: -1,
            start: Local::now(),
            workspace_id,
            project_id,
        };
        let mut res = client
            .post(format!(
                "{}/api/v9/workspaces/{}/time_entries",
                Client::BASE_URL,
                entry.workspace_id
            ))
            .body_json(&entry)?
            .send()
            .await?;
        Client::check_status(&mut res).await
    }

    pub async fn duplicate(self, client: &Client) -> NetResult<()> {
        Self::create_running(
            self.description,
            self.workspace_id,
            self.project_id,
            client,
        )
        .await
    }
}

#[derive(Clone, Debug, Serialize)]
struct CreateTimeEntry {
    created_with: String,
    description: Option<String>,
    duration: i64,
    #[serde(serialize_with = "datetime_serialize_utc")]
    start: DateTime<Local>,
    workspace_id: WorkspaceId,
    project_id: Option<ProjectId>,
}

#[derive(Clone, Debug)]
pub enum TimeEntryMessage {
    Edit(TimeEntry),
    Duplicate(TimeEntry),
}

impl TimeEntry {
    pub fn get_duration(&self) -> Duration {
        self.stop.unwrap_or_else(|| {
            Local::now().with_timezone(&self.start.timezone())
        }) - self.start
    }

    pub fn duration_string(&self) -> String {
        duration_to_hms(&self.get_duration())
    }

    pub fn project(&self, projects: &[Project]) -> MaybeProject {
        projects
            .iter()
            .find(|p| Some(p.id) == self.project_id)
            .cloned()
            .into()
    }

    pub fn description_text(&self) -> String {
        self.description
            .clone()
            .unwrap_or_else(|| "<NO DESCRIPTION>".to_string())
    }

    pub fn view(&self, projects: &[Project]) -> Element<TimeEntryMessage> {
        let project = self.project(projects);
        let name = self.description_text();
        button(
            row![
                column![
                    text(name)
                        .width(Length::Fill)
                        .wrapping(text::Wrapping::None),
                    project.project_badge()
                ],
                icon_button(iced_fonts::Bootstrap::Copy)
                    .style(button::primary)
                    .on_press_with(|| TimeEntryMessage::Duplicate(self.clone()))
                    .width(28),
                text(self.duration_string()).width(Length::Fixed(50f32))
            ]
            .spacing(10)
            .padding(iced::Padding {
                right: 10f32,
                ..iced::Padding::default()
            })
            .align_y(Vertical::Center),
        )
        .on_press_with(|| TimeEntryMessage::Edit(self.clone()))
        .clip(true)
        .style(button::text)
        .into()
    }
}

#[cfg(test)]
mod test {
    use super::TimeEntry;
    use crate::utils::Client;

    fn test_client() -> Client {
        Client::from_email_password(
            &std::env::var("TEST_EMAIL").expect("Please pass TEST_EMAIL"),
            &std::env::var("TEST_PASSWORD").expect("Please pass TEST_PASSWORD"),
        )
    }

    #[async_std::test]
    async fn test_load_until_now() {
        let client = test_client();
        let entries = TimeEntry::load(None, &client).await.expect("Failed");
        assert_ne!(entries.len(), 0);

        let prev_entries =
            TimeEntry::load(entries.last().map(|e| e.start), &client)
                .await
                .expect("Failed");
        assert_ne!(prev_entries.len(), 0);
        assert_ne!(prev_entries.first(), entries.last());
    }
}
