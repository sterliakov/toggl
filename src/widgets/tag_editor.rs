use std::fmt::Debug;

use iced::alignment::Vertical;
use iced::widget::{
    button, container, row, scrollable, Column, Row, Text, TextInput,
};
use iced::{Border, Length};
use iced_aw::{drop_down, DropDown};

use crate::widgets::icon_button;

#[derive(Clone, Debug)]
pub enum TagEditorMessage {
    Select(String),
    Deselect(String),
    Dismiss,
    Toggle,
    EditNew(String),
    SubmitNew,
}

#[derive(Debug, Default)]
pub struct TagEditor {
    options: Vec<String>,
    selected: Vec<String>,
    expanded: bool,
    new_tag: String,
}

impl TagEditor {
    pub fn new(options: Vec<String>, selected: Vec<String>) -> Self {
        Self {
            options,
            selected,
            ..Self::default()
        }
    }
    pub fn update(&mut self, message: TagEditorMessage) {
        use TagEditorMessage::*;
        match message {
            Select(choice) => {
                if !self.selected.contains(&choice) {
                    self.selected.push(choice);
                }
            }
            Deselect(choice) => {
                self.selected.retain(|t| t != &choice);
                self.expanded = false;
            }
            Dismiss => self.expanded = false,
            Toggle => self.expanded = !self.expanded,
            EditNew(text) => self.new_tag = text,
            SubmitNew => {
                if !self.selected.contains(&self.new_tag) {
                    self.options.push(self.new_tag.clone());
                    self.selected.push(self.new_tag.clone());
                }
                self.new_tag = "".to_string();
            }
        }
    }

    pub fn view(&self) -> iced::Element<'_, TagEditorMessage> {
        Row::new()
            .push(Text::new("Tags:"))
            .extend(self.selected.iter().map(Self::tag_item))
            .push(self.picker())
            .spacing(6)
            .align_y(Vertical::Center)
            .wrap()
            .into()
    }

    pub fn get_value(&self) -> Vec<String> {
        self.selected.clone()
    }

    fn picker(&self) -> iced::Element<'_, TagEditorMessage> {
        let underlay = icon_button(iced_fonts::Bootstrap::Plus)
            .width(26.0)
            .style(button::secondary)
            .on_press(TagEditorMessage::Toggle);

        let choices = Column::new()
            .push(self.new_tag_entry())
            .extend(
                self.options
                    .iter()
                    .filter_map(|choice| self.maybe_choice_option(choice)),
            )
            .width(Length::Fill);

        let overlay = container(scrollable(choices))
            // FIXME: dropdown has some issues with overlay positioning,
            // without these severe restrictions it will overflow everywhere.
            .max_width(120)
            .max_height(320);

        let dropdown = DropDown::new(underlay, overlay, self.expanded)
            .on_dismiss(TagEditorMessage::Dismiss)
            .alignment(drop_down::Alignment::End)
            .width(Length::Fill)
            .height(Length::Fill)
            // position right on top of the button
            .offset(drop_down::Offset { x: -26.0, y: 0.0 });

        Column::new()
            .push(dropdown)
            .padding(0)
            .width(Length::Fill)
            .max_width(120)
            .into()
    }

    fn new_tag_entry(&self) -> iced::Element<'_, TagEditorMessage> {
        TextInput::new("New tag", &self.new_tag)
            .id("tag-editor")
            .width(Length::Fill)
            .on_input(TagEditorMessage::EditNew)
            .on_submit(TagEditorMessage::SubmitNew)
            .into()
    }

    fn maybe_choice_option<'a>(
        &'a self,
        choice: &'a String,
    ) -> Option<iced::Element<'a, TagEditorMessage>> {
        if self.selected.contains(choice)
            || !self.new_tag.is_empty()
                && !choice.to_lowercase().contains(&self.new_tag.to_lowercase())
        {
            None
        } else {
            Some(
                button(Text::new(choice.to_string()))
                    .style(button::secondary)
                    .on_press(TagEditorMessage::Select(choice.clone()))
                    .width(Length::Fill)
                    .into(),
            )
        }
    }

    fn tag_item(name: &String) -> iced::Element<'_, TagEditorMessage> {
        let tag_style = |theme: &iced::Theme| {
            let color = theme.extended_palette().secondary.weak;
            container::Style {
                background: Some(color.color.into()),
                text_color: Some(color.text),
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }
        };
        container(
            row![
                Text::new(name.clone()).center(),
                icon_button(iced_fonts::Bootstrap::X)
                    .style(button::text)
                    .width(12.0)
                    .padding(iced::Padding {
                        left: 3.0,
                        top: -4.0,
                        ..iced::Padding::default()
                    })
                    .on_press(TagEditorMessage::Deselect(name.to_string()))
            ]
            .spacing(2)
            .align_y(Vertical::Center),
        )
        .style(tag_style)
        .padding(iced::Padding {
            top: 7.0,
            bottom: 2.0,
            left: 8.0,
            right: 4.0,
        })
        .width(Length::Shrink)
        .into()
    }
}
