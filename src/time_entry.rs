use chrono::{DateTime, Duration, Local};
use iced::alignment::Vertical;
use iced::widget::{button, column, row, text};
use iced::{Element, Length};
use log::debug;
use serde::{Deserialize, Serialize, Serializer};

use crate::entities::{MaybeProject, Project, ProjectId, WorkspaceId};
use crate::utils::{duration_to_hms, maybe_vec_deserialize, Client, NetResult};
use crate::widgets::icon_button;

fn datetime_serialize_utc<S: Serializer>(
    x: &DateTime<Local>,
    s: S,
) -> Result<S::Ok, S::Error> {
    x.to_utc().serialize(s)
}

#[expect(clippy::ref_option)]
fn maybe_datetime_serialize_utc<S: Serializer>(
    x: &Option<DateTime<Local>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    x.map(|d| d.to_utc()).serialize(s)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct TimeEntry {
    pub description: Option<String>,
    pub duration: i64,
    pub id: u64,
    pub project_id: Option<ProjectId>,
    #[serde(serialize_with = "datetime_serialize_utc")]
    pub start: DateTime<Local>,
    #[serde(serialize_with = "maybe_datetime_serialize_utc")]
    pub stop: Option<DateTime<Local>>,
    #[serde(deserialize_with = "maybe_vec_deserialize")]
    pub tags: Vec<String>,
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

        let entries: Vec<Self> = client
            .get(format!("{}/api/v9/me/time_entries", Client::BASE_URL))
            .query(&QueryParams { before })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(entries)
    }

    pub fn split_running(all_entries: Vec<Self>) -> (Option<Self>, Vec<Self>) {
        match &*all_entries {
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

    pub async fn save(&self, client: &Client) -> NetResult<Self> {
        #[derive(Serialize)]
        struct UpdateRequest<'a> {
            #[serde(flatten)]
            entry: &'a TimeEntry,
            tag_action: &'a str,
        }

        debug!("Updating a time entry {}...", self.id);
        client
            .put(format!(
                "{}/api/v9/workspaces/{}/time_entries/{}",
                Client::BASE_URL,
                self.workspace_id,
                self.id
            ))
            .json(&UpdateRequest {
                entry: self,
                tag_action: "delete",
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    pub async fn stop(&self, client: &Client) -> NetResult<Self> {
        debug!("Stopping a time entry {}...", self.id);
        assert!(
            self.stop.is_none(),
            "Attempted to stop an already stopped entry"
        );
        client
            .patch(format!(
                "{}/api/v9/workspaces/{}/time_entries/{}/stop",
                Client::BASE_URL,
                self.workspace_id,
                self.id
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    pub async fn delete(&self, client: &Client) -> NetResult<()> {
        debug!("Deleting a time entry {}...", self.id);
        client
            .delete(format!(
                "{}/api/v9/workspaces/{}/time_entries/{}",
                Client::BASE_URL,
                self.workspace_id,
                self.id
            ))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn create_running(
        description: Option<String>,
        workspace_id: WorkspaceId,
        project_id: Option<ProjectId>,
        client: &Client,
    ) -> NetResult<Self> {
        debug!("Creating a time entry...");
        let entry = CreateTimeEntry {
            created_with: "ST-Toggl-Client".to_owned(),
            description,
            duration: -1,
            start: Local::now(),
            workspace_id,
            project_id,
        };
        client
            .post(format!(
                "{}/api/v9/workspaces/{}/time_entries",
                Client::BASE_URL,
                entry.workspace_id
            ))
            .json(&entry)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    pub async fn duplicate(self, client: &Client) -> NetResult<Self> {
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
            .unwrap_or_else(|| "<NO DESCRIPTION>".to_owned())
    }

    pub fn view(&self, projects: &[Project]) -> Element<'_, TimeEntryMessage> {
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
    #![allow(clippy::shadow_unrelated)]

    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::hash::Hash;

    use super::TimeEntry;
    use crate::test::test_client;
    use crate::ExtendedMe;

    #[tokio::test]
    async fn test_crud_cycle() {
        let client = test_client();
        let me = ExtendedMe::load(&client).await.expect("get self");
        let ws = me.workspaces.first().expect("no workspace").id;
        let initial_count = me.time_entries.len();

        TimeEntry::create_running(Some("Test".to_owned()), ws, None, &client)
            .await
            .expect("create");
        let entries =
            TimeEntry::load(None, &client).await.expect("get entries");
        assert_eq!(entries.len(), initial_count + 1);
        assert_eq!(entries[0].description, Some("Test".to_owned()));
        let (running, _) = TimeEntry::split_running(entries);
        assert!(running.is_some());

        let mut last = {
            running.unwrap().stop(&client).await.expect("stop");
            let entries =
                TimeEntry::load(None, &client).await.expect("get entries");
            assert_eq!(entries.len(), initial_count + 1);
            assert_eq!(entries[0].description, Some("Test".to_owned()));
            let (running, entries) = TimeEntry::split_running(entries);
            assert!(running.is_none());
            entries[0].clone()
        };

        // Respect API limits
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut last = {
            last.description = Some("Other".to_owned());
            last.save(&client).await.expect("update");
            let entries =
                TimeEntry::load(None, &client).await.expect("get entries");
            assert_eq!(entries.len(), initial_count + 1);
            assert_eq!(entries[0].description, Some("Other".to_owned()));
            entries[0].clone()
        };

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut last = {
            last.tags = vec!["foo".to_owned(), "bar".to_owned()];
            last.save(&client).await.expect("update");
            let entries =
                TimeEntry::load(None, &client).await.expect("get entries");
            assert_ignore_order(
                entries[0].tags.clone(),
                vec!["foo".to_owned(), "bar".to_owned()],
            );
            entries[0].clone()
        };

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let last = {
            last.tags = vec!["foo".to_owned(), "baz".to_owned()];
            last.save(&client).await.expect("update");
            let entries =
                TimeEntry::load(None, &client).await.expect("get entries");
            assert_ignore_order(
                entries[0].tags.clone(),
                vec!["foo".to_owned(), "baz".to_owned()],
            );
            entries[0].clone()
        };

        let last = {
            // delete
            last.delete(&client).await.expect("delete");
            let entries =
                TimeEntry::load(None, &client).await.expect("get entries");
            assert_eq!(entries.len(), initial_count);
            entries.last().cloned()
        };

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        TimeEntry::load(last.map(|e| e.start), &client)
            .await
            .expect("get older");
    }

    fn assert_ignore_order<T: Eq + Hash + Debug>(
        got: impl IntoIterator<Item = T>,
        expected: impl IntoIterator<Item = T>,
    ) {
        let got: HashSet<_> = got.into_iter().collect();
        let expected: HashSet<_> = expected.into_iter().collect();
        assert_eq!(got, expected);
    }
}
