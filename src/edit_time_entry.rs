use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input,
};
use iced::{Element, Fill, Length, Right, Task as Command};
use time::{format_description, OffsetDateTime, PrimitiveDateTime};

use crate::client::Client;
use crate::time_entry::TimeEntry;

#[derive(Debug)]
pub struct EditTimeEntry {
    entry: TimeEntry,
    api_token: String,
    description_content: text_editor::Content,
    start_text: String,
    stop_text: String,
    error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum EditTimeEntryMessage {
    DescriptionEdited(text_editor::Action),
    StartEdited(String),
    StopEdited(String),
    Submit,
    Delete,
    Completed,
    Error(String),
}

impl EditTimeEntry {
    pub fn new(entry: TimeEntry, api_token: &str) -> Self {
        let description = entry.description.clone();
        let start_text = date_as_human_readable(&Some(entry.start));
        let stop_text = date_as_human_readable(&entry.stop);
        Self {
            entry,
            api_token: api_token.to_string(),
            description_content: text_editor::Content::with_text(
                &description.unwrap_or("".to_string()),
            ),
            start_text,
            stop_text,
            error: None,
        }
    }

    pub fn view(&self) -> Element<EditTimeEntryMessage> {
        let content = column![
            column![
                button("X")
                    .on_press(EditTimeEntryMessage::Completed)
                    .style(button::text),
            ].align_x(Right).width(Fill),
            text_editor(&self.description_content)
                .on_action(EditTimeEntryMessage::DescriptionEdited),
            row![
                text_input("Start", &self.start_text)
                    .id("start-input")
                    .on_input(EditTimeEntryMessage::StartEdited),
                text_input("Stop", &self.stop_text)
                    .id("end-input")
                    .on_input(EditTimeEntryMessage::StopEdited),
            ]
            .spacing(20),
            row![
                button("Save")
                    .on_press(EditTimeEntryMessage::Submit)
                    .style(button::primary)
                    .width(Length::FillPortion(1)),
                button("Delete")
                    .on_press(EditTimeEntryMessage::Delete)
                    .style(button::danger)
                    .width(Length::FillPortion(1)),
            ].spacing(20),
        ]
        .push_maybe(self.error.clone().map(|e| text(e).style(text::danger)))
        .spacing(10);

        scrollable(container(content).center_x(Fill).padding(40)).into()
    }

    pub fn update(
        &mut self,
        message: EditTimeEntryMessage,
    ) -> Command<EditTimeEntryMessage> {
        match &message {
            EditTimeEntryMessage::DescriptionEdited(action) => {
                self.description_content.perform(action.clone());
                self.entry.description = Some(self.description_content.text());
            }
            EditTimeEntryMessage::StartEdited(start) => {
                self.start_text = start.clone();
            }
            EditTimeEntryMessage::StopEdited(stop) => {
                self.stop_text = stop.clone();
            }
            EditTimeEntryMessage::Submit => {
                {
                    let Ok(maybe_date) = date_from_human_readable(
                        &self.start_text,
                        &self.entry.start,
                    ) else {
                        return Command::done(EditTimeEntryMessage::Error(
                            format!("Invalid start date: {}", self.start_text),
                        ));
                    };
                    let Some(date) = maybe_date else {
                        return Command::done(EditTimeEntryMessage::Error(
                            "Start cannot be blank".to_string(),
                        ));
                    };
                    self.entry.start = date;
                }
                {
                    let Ok(date) = date_from_human_readable(
                        &self.stop_text,
                        &self.entry.start,
                    ) else {
                        return Command::done(EditTimeEntryMessage::Error(
                            format!("Invalid end date: {}", self.stop_text),
                        ));
                    };
                    self.entry.stop = date;
                }
                self.entry.duration = self
                    .entry
                    .stop
                    .map(|stop| (stop - self.entry.start).whole_seconds())
                    .unwrap_or(-1);
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
            EditTimeEntryMessage::Completed => {}
            EditTimeEntryMessage::Error(err) => {
                self.error = Some(err.clone());
            }
        };
        Command::none()
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

const FORMAT: &str = "[day]/[month]/[year] [hour]:[minute]:[second]";

fn date_as_human_readable(date: &Option<OffsetDateTime>) -> String {
    if let Some(date) = date {
        let format = format_description::parse_borrowed::<2>(FORMAT).unwrap();
        date.format(&format).unwrap()
    } else {
        "".to_string()
    }
}

fn date_from_human_readable(
    new_input: &str,
    old_date: &OffsetDateTime,
) -> time::Result<Option<OffsetDateTime>> {
    if new_input.is_empty() {
        return Ok(None);
    }
    let format = format_description::parse_borrowed::<2>(FORMAT).unwrap();
    let new_date = PrimitiveDateTime::parse(new_input, &format)?;
    Ok(Some(new_date.assume_offset(old_date.offset())))
}
