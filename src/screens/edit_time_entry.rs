use iced::keyboard::key::Named as NamedKey;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text,
};
use iced::{keyboard, Element, Fill, Length, Task as Command};

use crate::entities::MaybeProject;
use crate::state::{EntryEditAction, EntryEditInfo, State};
use crate::time_entry::TimeEntry;
use crate::utils::{Client, ExactModifiers as _};
use crate::widgets::{
    close_button, CustomWidget, DateTimeEditMessage, DateTimeWidget, TagEditor,
    TagEditorMessage, TextEditorExt, TextEditorMessage,
};

#[derive(Debug)]
pub struct EditTimeEntry {
    entry: TimeEntry,
    api_token: String,
    description_editor: TextEditorExt,
    start_dt: DateTimeWidget,
    stop_dt: DateTimeWidget,
    error: Option<String>,
    projects: Vec<MaybeProject>,
    selected_project: MaybeProject,
    tag_editor: TagEditor,
}

#[derive(Clone, Debug)]
pub enum EditTimeEntryMessage {
    DescriptionEdited(TextEditorMessage),
    TagsEdited(TagEditorMessage),
    ProjectSelected(MaybeProject),
    StartEdited(DateTimeEditMessage),
    StopEdited(DateTimeEditMessage),
    Submit,
    Delete,
    Abort,
    Completed(EntryEditInfo),
    Error(String),
}

impl CustomWidget<EditTimeEntryMessage> for EditTimeEntry {
    fn view(&self, state: &State) -> Element<'_, EditTimeEntryMessage> {
        use std::borrow::Borrow as _;

        let content = column![
            close_button(EditTimeEntryMessage::Abort),
            Element::from(self.description_editor.view(state))
                .map(EditTimeEntryMessage::DescriptionEdited),
            row![
                self.start_dt
                    .view(state)
                    .map(EditTimeEntryMessage::StartEdited),
                self.stop_dt
                    .view(state)
                    .map(EditTimeEntryMessage::StopEdited),
            ]
            .spacing(20),
            pick_list(
                self.projects.borrow(),
                Some(self.selected_project.clone()),
                EditTimeEntryMessage::ProjectSelected
            )
            .style(|theme, status| {
                let mut base = pick_list::default(theme, status);
                match &self.selected_project {
                    MaybeProject::Some(p) => {
                        base.background = p.parsed_color().into();
                    }
                    MaybeProject::None => {}
                }
                base
            }),
            self.tag_editor
                .view(state)
                .map(EditTimeEntryMessage::TagsEdited),
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

    fn update(
        &mut self,
        message: EditTimeEntryMessage,
        state: &State,
    ) -> Command<EditTimeEntryMessage> {
        use EditTimeEntryMessage::*;

        match message {
            DescriptionEdited(action) => {
                return self
                    .description_editor
                    .update(action, state)
                    .map(DescriptionEdited);
            }
            TagsEdited(action) => {
                return self.tag_editor.update(action, state).map(TagsEdited);
            }
            StartEdited(DateTimeEditMessage::Finish)
            | StopEdited(DateTimeEditMessage::Finish) => {
                return Command::done(Submit);
            }
            StartEdited(start) => {
                return self.start_dt.update(start, state).map(StartEdited)
            }
            StopEdited(stop) => {
                return self.stop_dt.update(stop, state).map(StartEdited)
            }
            ProjectSelected(p) => {
                self.entry.project_id = p.id();
                self.selected_project = p;
            }
            Submit => {
                match self.start_dt.get_value() {
                    Err(e) => return Command::done(Error(e)),
                    Ok(None) => {
                        return Command::done(Error(
                            "Start cannot be blank".to_owned(),
                        ))
                    }
                    Ok(Some(date)) => self.entry.start = date,
                }
                match self.stop_dt.get_value() {
                    Ok(stop) => self.entry.stop = stop,
                    Err(e) => {
                        return Command::done(Error(e));
                    }
                }
                let duration = self
                    .entry
                    .stop
                    .map(|stop| (stop - self.entry.start).num_seconds());
                if duration.unwrap_or(1) < 0 {
                    return Command::done(Error(
                        "Start must come before end!".to_owned(),
                    ));
                }
                self.entry.duration = duration.unwrap_or(-1);
                self.entry.description =
                    Some(self.description_editor.get_value());
                self.entry.tags = self.tag_editor.get_value();
                return Command::future(Self::submit(
                    self.entry.clone(),
                    self.api_token.clone(),
                ));
            }
            Delete => {
                return Command::future(Self::delete(
                    self.entry.clone(),
                    self.api_token.clone(),
                ));
            }
            Abort | Completed(_) => {}
            Error(err) => {
                self.error = Some(err);
            }
        }
        Command::none()
    }

    fn handle_key(
        &mut self,
        key: NamedKey,
        modifiers: keyboard::Modifiers,
    ) -> Option<Command<EditTimeEntryMessage>> {
        if let Some(c) = self.start_dt.handle_key(key, modifiers) {
            Some(c.map(EditTimeEntryMessage::StartEdited))
        } else if let Some(c) = self.stop_dt.handle_key(key, modifiers) {
            Some(c.map(EditTimeEntryMessage::StopEdited))
        } else if matches!(key, NamedKey::Enter)
            && modifiers.is_exact_ctrl_or_cmd()
        {
            Some(Command::done(EditTimeEntryMessage::Submit))
        } else if matches!(key, NamedKey::Escape) && modifiers.is_empty() {
            Some(Command::done(EditTimeEntryMessage::Abort))
        } else {
            Some(Command::none())
        }
    }
}

impl EditTimeEntry {
    pub fn new(entry: TimeEntry, state: &State) -> Self {
        let description = entry.description.clone();
        let start_dt = DateTimeWidget::new(
            Some(entry.start),
            "Start",
            "start-input",
            state.customization(),
        );
        let stop_dt = DateTimeWidget::new(
            entry.stop,
            "Stop",
            "stop-input",
            state.customization(),
        );
        let profile = state.current_profile();
        let selected_project = entry.project(&profile.projects);
        let projects: Vec<MaybeProject> = std::iter::once(MaybeProject::None)
            .chain(
                profile
                    .projects
                    .iter()
                    .cloned()
                    .map(std::convert::Into::into),
            )
            .collect();
        let tags = entry.tags.clone();
        Self {
            entry,
            api_token: state.api_token(),
            description_editor: TextEditorExt::new(description.as_ref()),
            start_dt,
            stop_dt,
            error: None,
            projects,
            selected_project,
            tag_editor: TagEditor::new(
                profile.tags.iter().map(|t| t.name.clone()).collect(),
                tags,
            ),
        }
    }

    async fn submit(
        entry: TimeEntry,
        api_token: String,
    ) -> EditTimeEntryMessage {
        let client = &Client::from_api_token(&api_token);
        match entry.save(client).await {
            Err(e) => EditTimeEntryMessage::Error(e.to_string()),
            Ok(new_entry) => EditTimeEntryMessage::Completed(EntryEditInfo {
                entry: new_entry,
                action: EntryEditAction::Update,
            }),
        }
    }

    async fn delete(
        entry: TimeEntry,
        api_token: String,
    ) -> EditTimeEntryMessage {
        let client = &Client::from_api_token(&api_token);
        if let Err(message) = entry.delete(client).await {
            EditTimeEntryMessage::Error(message.to_string())
        } else {
            EditTimeEntryMessage::Completed(EntryEditInfo {
                entry,
                action: EntryEditAction::Delete,
            })
        }
    }
}
