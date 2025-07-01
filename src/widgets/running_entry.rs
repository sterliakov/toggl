use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Color, Length, Task as Command};
use log::{info, warn};

use super::CustomWidget;
use crate::state::{EntryEditAction, EntryEditInfo, State};
use crate::time_entry::TimeEntry;
use crate::utils::Client;
use crate::widgets::icon_button;

#[derive(Clone, Debug, Default)]
pub struct RunningEntry {
    draft_description: String,
}

#[derive(Clone, Debug)]
pub enum RunningEntryMessage {
    // Private
    Create,
    EditDraft(String),
    Stop,
    SubmitOk(Box<TimeEntry>),
    // Public
    StartEditing(Box<TimeEntry>),
    Error(String),
    SyncUpdate(EntryEditInfo),
}

impl CustomWidget<RunningEntryMessage> for RunningEntry {
    fn view(&self, state: &State) -> iced::Element<'_, RunningEntryMessage> {
        let Some(entry) = state.running_entry.clone() else {
            return self.new_entry_input();
        };

        let project = entry.project(&state.projects);
        let name = entry.description_text();
        let duration = entry.duration_string();
        container(
            row![
                button(text(name).wrapping(text::Wrapping::None))
                    .width(Length::Fill)
                    .style(|_, _| button::Style {
                        text_color: Color::WHITE,
                        ..button::Style::default()
                    })
                    .on_press_with(move || RunningEntryMessage::StartEditing(
                        Box::new(entry.clone())
                    ))
                    .clip(true),
                column![
                    text(duration).width(Length::Fixed(50f32)),
                    container(project.project_badge()),
                ]
                .align_x(Horizontal::Right)
                .padding([4, 0]),
                icon_button(iced_fonts::Bootstrap::Pause)
                    .style(button::primary)
                    .on_press(RunningEntryMessage::Stop)
                    .width(Length::Fixed(28.0)),
            ]
            .spacing(10)
            .padding(iced::Padding {
                right: 10.0,
                top: 4.0,
                bottom: 4.0,
                left: 0.0,
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

    fn update(
        &mut self,
        message: RunningEntryMessage,
        state: &State,
    ) -> Command<RunningEntryMessage> {
        use RunningEntryMessage::*;
        match message {
            Create => {
                let token = state.api_token.clone();
                let description = self.draft_description.clone();
                let Some(workspace_id) = state.default_workspace else {
                    return Command::done(Error(
                        "No workspace selected!".to_owned(),
                    ));
                };
                let project_id = state.default_project;
                Command::future(async move {
                    let client = Client::from_api_token(&token);
                    let fut = TimeEntry::create_running(
                        Some(description),
                        workspace_id,
                        project_id,
                        &client,
                    );
                    match fut.await {
                        Err(e) => Error(e.to_string()),
                        Ok(entry) => SubmitOk(Box::new(entry)),
                    }
                })
            }
            SubmitOk(entry) => {
                self.draft_description = String::new();
                Command::done(SyncUpdate(EntryEditInfo {
                    action: EntryEditAction::Create,
                    entry: *entry,
                }))
            }
            EditDraft(text) => {
                self.draft_description = text;
                Command::none()
            }
            Stop => {
                if let Some(entry) = state.running_entry.clone() {
                    info!("Stopping running entry {}...", entry.id);
                    let token = state.api_token.clone();
                    Command::future(async move {
                        let client = Client::from_api_token(&token);
                        match entry.stop(&client).await {
                            Err(e) => Error(e.to_string()),
                            Ok(entry) => SyncUpdate(EntryEditInfo {
                                action: EntryEditAction::Update,
                                entry,
                            }),
                        }
                    })
                } else {
                    warn!("Requested to stop a nonexistent running entry.");
                    Command::none()
                }
            }
            _ => Command::none(),
        }
    }
}

impl RunningEntry {
    fn new_entry_input(&self) -> iced::Element<'_, RunningEntryMessage> {
        row![
            text_input("Create new entry...", &self.draft_description)
                .id("running-entry-input")
                .style(|_, _| text_input::Style {
                    background: iced::color!(0x161616).into(),
                    border: iced::Border::default(),
                    icon: Color::WHITE,
                    placeholder: iced::color!(0xd8d8d8),
                    value: Color::WHITE,
                    selection: Color::WHITE,
                })
                .on_input(RunningEntryMessage::EditDraft)
                .on_submit(RunningEntryMessage::Create),
            button("Create").on_press(RunningEntryMessage::Create)
        ]
        .into()
    }
}
