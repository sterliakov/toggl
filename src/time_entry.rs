use chrono::{DateTime, Duration, Local};
use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length};
use iced_aw::badge;
use log::debug;
use serde::{Deserialize, Serialize, Serializer};

use crate::client::{Client, Result as NetResult};
use crate::project::{Project, ProjectId};
use crate::workspace::WorkspaceId;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    // pub async fn load(client: &Client) -> NetResult<(Option<Self>, Vec<Self>)> {
    //     let all_entries = client
    //         .get([Client::BASE_URL, "/api/v9/me/time_entries"].join(""))
    //         .send()
    //         .await?
    //         .body_json::<Vec<Self>>()
    //         .await?;
    //     Ok(Self::split_running(all_entries))
    // }

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
            .put(
                [
                    Client::BASE_URL,
                    &format!(
                        "/api/v9/workspaces/{}/time_entries/{}",
                        self.workspace_id, self.id
                    ),
                ]
                .join(""),
            )
            .body_json(&self)?
            .send()
            .await?;
        Client::check_status(&mut res).await
    }

    pub async fn stop(&self, client: &Client) -> NetResult<()> {
        debug!("Stopping a time entry {}...", self.id);
        assert!(self.stop.is_none());
        let mut res = client
            .patch(
                [
                    Client::BASE_URL,
                    &format!(
                        "/api/v9/workspaces/{}/time_entries/{}/stop",
                        self.workspace_id, self.id
                    ),
                ]
                .join(""),
            )
            .send()
            .await?;
        Client::check_status(&mut res).await
    }

    pub async fn delete(self, client: &Client) -> NetResult<()> {
        debug!("Deleting a time entry {}...", self.id);
        let mut res = client
            .delete(
                [
                    Client::BASE_URL,
                    &format!(
                        "/api/v9/workspaces/{}/time_entries/{}",
                        self.workspace_id, self.id
                    ),
                ]
                .join(""),
            )
            .send()
            .await?;
        Client::check_status(&mut res).await
    }

    fn duration_string(&self) -> String {
        let diff = self
            .stop
            .unwrap_or(Local::now().with_timezone(&self.start.timezone()))
            - self.start;
        duration_to_hms(&diff)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateTimeEntry {
    created_with: String,
    description: Option<String>,
    duration: i64,
    start: DateTime<Local>,
    workspace_id: WorkspaceId,
    project_id: Option<ProjectId>,
}

impl CreateTimeEntry {
    pub fn new(
        description: Option<String>,
        workspace_id: WorkspaceId,
        project_id: Option<ProjectId>,
    ) -> Self {
        Self {
            created_with: "ST-Toggl-Client".to_string(),
            description,
            duration: -1,
            start: Local::now(),
            workspace_id,
            project_id,
        }
    }

    pub async fn create(&self, client: &Client) -> NetResult<()> {
        debug!("Creating a time entry...");
        let mut res = client
            .post(
                [
                    Client::BASE_URL.to_string(),
                    format!(
                        "/api/v9/workspaces/{}/time_entries",
                        self.workspace_id
                    ),
                ]
                .join(""),
            )
            .body_json(&self)?
            .send()
            .await?;
        Client::check_status(&mut res).await
    }
}

#[derive(Clone, Debug)]
pub enum TimeEntryMessage {
    Edit(usize),
    EditRunning,
    StopRunning,
    Duplicate(Box<TimeEntry>),
}

impl TimeEntry {
    pub fn view(
        &self,
        i: usize,
        projects: &[Project],
    ) -> Element<TimeEntryMessage> {
        let project = projects.iter().find(|p| Some(p.id) == self.project_id);
        let name = self
            .description
            .clone()
            .unwrap_or("<NO DESCRIPTION>".to_string());
        let project_badge = if let Some(project) = project {
            let color = Color::parse(&project.color)
                .expect("Project color must be valid");
            badge::Badge::new(text(project.name.clone()).size(12)).style(
                move |_, _| badge::Style {
                    background: color.into(),
                    ..badge::Style::default()
                },
            )
        } else {
            badge(text("No project".to_string()).size(12))
                .style(iced_aw::style::badge::light)
        };
        button(
            row![
                column![
                    text(name)
                        .width(Length::Fill)
                        .wrapping(text::Wrapping::None),
                    project_badge
                ],
                button("+")
                    .style(button::primary)
                    .on_press_with(|| TimeEntryMessage::Duplicate(Box::new(
                        self.clone()
                    )))
                    .width(Length::Shrink),
                text(self.duration_string()).width(Length::Fixed(60f32))
            ]
            .spacing(10)
            .padding(iced::Padding {
                right: 10f32,
                ..iced::Padding::default()
            })
            .align_y(Vertical::Center),
        )
        .on_press(TimeEntryMessage::Edit(i))
        .clip(true)
        .style(button::text)
        .into()
    }

    pub fn view_running(&self) -> Element<TimeEntryMessage> {
        let name = self
            .description
            .clone()
            .unwrap_or("<NO DESCRIPTION>".to_string());
        container(
            row![
                button(text(name).wrapping(text::Wrapping::None))
                    .width(Length::Fill)
                    .style(|_, _| button::Style {
                        text_color: Color::WHITE,
                        ..button::Style::default()
                    })
                    .on_press(TimeEntryMessage::EditRunning)
                    .clip(true),
                text(self.duration_string()).width(Length::Fixed(60f32)),
                button("Stop")
                    .style(button::primary)
                    .on_press(TimeEntryMessage::StopRunning)
                    .width(Length::Fixed(60f32)),
            ]
            .spacing(10)
            .padding(iced::Padding {
                right: 10f32,
                ..iced::Padding::default()
            })
            .align_y(Vertical::Center),
        )
        .style(|_| container::Style {
            background: Some(iced::color!(0x161616).into()),
            text_color: Some(Color::WHITE),
            ..container::Style::default()
        })
        .into()
    }
}

fn duration_to_hms(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
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
        let (_, entries) = TimeEntry::load(&client).await.expect("Failed");
        assert_ne!(entries.len(), 0);
    }
}
