use iced::alignment::Horizontal;
use iced::keyboard::key::Named as NamedKey;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_editor,
};
use iced::{keyboard, Element, Fill, Length, Task as Command};
use iced_fonts::bootstrap::Bootstrap;

use crate::customization::Customization;
use crate::project::{MaybeProject, Project};
use crate::time_entry::TimeEntry;
use crate::utils::{Client, ExactModifiers};
use crate::widgets::{icon_text, DateTimeEditMessage, DateTimeWidget};

#[derive(Debug)]
pub struct EditTimeEntry {
    entry: TimeEntry,
    api_token: String,
    description_content: text_editor::Content,
    start_dt: DateTimeWidget,
    stop_dt: DateTimeWidget,
    error: Option<String>,
    projects: Vec<MaybeProject>,
    selected_project: MaybeProject,
}

#[derive(Clone, Debug)]
pub enum EditTimeEntryMessage {
    DescriptionEdited(text_editor::Action),
    ProjectSelected(MaybeProject),
    StartEdited(DateTimeEditMessage),
    StopEdited(DateTimeEditMessage),
    Submit,
    Delete,
    Abort,
    Completed,
    Error(String),
}

impl EditTimeEntry {
    pub fn new(
        entry: TimeEntry,
        api_token: &str,
        customization: &Customization,
        projects: Vec<Project>,
    ) -> Self {
        let description = entry.description.clone();
        let start_dt = DateTimeWidget::new(
            Some(entry.start),
            "Start",
            "start-input",
            customization,
        );
        let stop_dt = DateTimeWidget::new(
            entry.stop,
            "Stop",
            "stop-input",
            customization,
        );
        let selected_project = projects
            .iter()
            .find(|p| Some(p.id) == entry.project_id)
            .cloned();
        Self {
            entry,
            api_token: api_token.to_string(),
            description_content: text_editor::Content::with_text(
                &description.unwrap_or("".to_string()),
            ),
            start_dt,
            stop_dt,
            error: None,
            projects: projects.into_iter().map(|p| p.into()).collect(),
            selected_project: selected_project.into(),
        }
    }

    pub fn view(
        &self,
        customization: &Customization,
    ) -> Element<EditTimeEntryMessage> {
        let content = column![
            container(
                button(
                    icon_text(Bootstrap::X)
                        .size(24)
                        .width(iced::Length::Shrink)
                )
                .on_press(EditTimeEntryMessage::Abort)
                .style(button::text)
            )
            .align_x(Horizontal::Right)
            .width(iced::Length::Fill),
            text_editor(&self.description_content)
                .on_action(EditTimeEntryMessage::DescriptionEdited)
                .key_binding(|press| {
                    use text_editor::{Binding, Motion};

                    match press.key.as_ref() {
                        keyboard::Key::Named(NamedKey::Backspace)
                        | keyboard::Key::Character("w")
                            if press.modifiers.is_exact_ctrl_or_cmd() =>
                        {
                            Some(Binding::Sequence(vec![
                                Binding::SelectWord,
                                Binding::Backspace,
                                Binding::Backspace, // Preceding whitespace if any
                            ]))
                        }
                        keyboard::Key::Named(NamedKey::Delete)
                            if press.modifiers.is_exact_ctrl_or_cmd() =>
                        {
                            Some(Binding::Sequence(vec![
                                Binding::Select(Motion::WordRight),
                                Binding::Delete,
                            ]))
                        }
                        keyboard::Key::Character("e")
                            if press.modifiers.is_exact_ctrl_or_cmd() =>
                        {
                            Some(Binding::Move(Motion::DocumentEnd))
                        }
                        // Propagate Ctrl+Enter up
                        keyboard::Key::Named(NamedKey::Enter)
                            if press.modifiers.is_exact_ctrl_or_cmd() =>
                        {
                            None
                        }
                        _ => Binding::from_key_press(press),
                    }
                }),
            row![
                self.start_dt
                    .view(customization)
                    .map(EditTimeEntryMessage::StartEdited),
                self.stop_dt
                    .view(customization)
                    .map(EditTimeEntryMessage::StopEdited),
            ]
            .spacing(20),
            pick_list(
                std::iter::once(MaybeProject::None)
                    .chain(self.projects.clone().into_iter())
                    .collect::<Vec<_>>(),
                Some(self.selected_project.clone()),
                EditTimeEntryMessage::ProjectSelected
            )
            .style(|theme, status| {
                let base = pick_list::default(theme, status);
                pick_list::Style {
                    background: match &self.selected_project {
                        MaybeProject::Some(p) => iced::Color::parse(&p.color)
                            .expect("Must be a valid color")
                            .into(),
                        MaybeProject::None => base.background,
                    },
                    ..base
                }
            }),
            row![
                button("Save")
                    .on_press(EditTimeEntryMessage::Submit)
                    .style(button::primary)
                    .width(Length::FillPortion(1)),
                button("Delete")
                    .on_press(EditTimeEntryMessage::Delete)
                    .style(button::danger)
                    .width(Length::FillPortion(1)),
            ]
            .spacing(20),
        ]
        .push_maybe(self.error.clone().map(|e| text(e).style(text::danger)))
        .spacing(10);

        scrollable(container(content).center_x(Fill).padding(10)).into()
    }

    pub fn update(
        &mut self,
        message: EditTimeEntryMessage,
        customization: &Customization,
    ) -> Command<EditTimeEntryMessage> {
        match message {
            EditTimeEntryMessage::DescriptionEdited(action) => {
                self.description_content.perform(action);
                self.entry.description = Some(self.description_content.text());
            }
            EditTimeEntryMessage::StartEdited(start) => {
                return self
                    .start_dt
                    .update(start, customization)
                    .map(EditTimeEntryMessage::StartEdited)
            }
            EditTimeEntryMessage::StopEdited(stop) => {
                return self
                    .stop_dt
                    .update(stop, customization)
                    .map(EditTimeEntryMessage::StartEdited)
            }
            EditTimeEntryMessage::ProjectSelected(p) => {
                self.entry.project_id = match &p {
                    MaybeProject::Some(p) => Some(p.id),
                    MaybeProject::None => None,
                };
                self.selected_project = p;
            }
            EditTimeEntryMessage::Submit => {
                match self.start_dt.get_value() {
                    Err(e) => {
                        return Command::done(EditTimeEntryMessage::Error(e))
                    }
                    Ok(None) => {
                        return Command::done(EditTimeEntryMessage::Error(
                            "Start cannot be blank".to_string(),
                        ))
                    }
                    Ok(Some(date)) => self.entry.start = date,
                };
                match self.stop_dt.get_value() {
                    Ok(stop) => self.entry.stop = stop,
                    Err(e) => {
                        return Command::done(EditTimeEntryMessage::Error(e));
                    }
                };
                let duration = self
                    .entry
                    .stop
                    .map(|stop| (stop - self.entry.start).num_seconds());
                if duration.unwrap_or(1) < 0 {
                    return Command::done(EditTimeEntryMessage::Error(
                        "Start must come before end!".to_string(),
                    ));
                };
                self.entry.duration = duration.unwrap_or(-1);
                return Command::future(Self::submit(
                    self.entry.clone(),
                    self.api_token.clone(),
                ));
            }
            EditTimeEntryMessage::Delete => {
                return Command::future(Self::delete(
                    self.entry.clone(),
                    self.api_token.clone(),
                ));
            }
            EditTimeEntryMessage::Abort => {}
            EditTimeEntryMessage::Completed => {}
            EditTimeEntryMessage::Error(err) => {
                self.error = Some(err);
            }
        };
        Command::none()
    }

    pub fn handle_key(
        &mut self,
        key: NamedKey,
        modifiers: keyboard::Modifiers,
    ) -> Option<Command<EditTimeEntryMessage>> {
        if let Some(c) = self.start_dt.handle_key(key) {
            Some(c.map(EditTimeEntryMessage::StartEdited))
        } else if let Some(c) = self.stop_dt.handle_key(key) {
            Some(c.map(EditTimeEntryMessage::StopEdited))
        } else if matches!(key, NamedKey::Enter)
            && modifiers.is_exact_ctrl_or_cmd()
        {
            Some(Command::done(EditTimeEntryMessage::Submit))
        } else {
            None
        }
    }

    async fn submit(
        entry: TimeEntry,
        api_token: String,
    ) -> EditTimeEntryMessage {
        let client = &Client::from_api_token(&api_token);
        if let Err(message) =
            entry.save(client).await.map_err(|e| e.to_string())
        {
            EditTimeEntryMessage::Error(message)
        } else {
            EditTimeEntryMessage::Completed
        }
    }

    async fn delete(
        entry: TimeEntry,
        api_token: String,
    ) -> EditTimeEntryMessage {
        let client = &Client::from_api_token(&api_token);
        if let Err(message) =
            entry.delete(client).await.map_err(|e| e.to_string())
        {
            EditTimeEntryMessage::Error(message)
        } else {
            EditTimeEntryMessage::Completed
        }
    }
}
