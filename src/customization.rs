use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use iced::widget::{button, text};
use iced::Task as Command;
use iced_aw::menu;
use serde::{Deserialize, Serialize};

use crate::components::menu_button;

trait LocaleString {
    fn to_format_string(&self) -> String;
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Default, Serialize, Deserialize,
)]
pub enum DateFormat {
    #[default]
    Dmy,
    Mdy,
}
impl LocaleString for DateFormat {
    fn to_format_string(&self) -> String {
        match self {
            DateFormat::Dmy => "%d-%m-%y".to_string(),
            DateFormat::Mdy => "%m-%d-%y".to_string(),
        }
    }
}
impl std::fmt::Display for DateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            DateFormat::Dmy => "dd-mm-yyyy",
            DateFormat::Mdy => "mm-dd-yyyy",
        };
        f.write_str(repr)
    }
}
impl DateFormat {
    const VALUES: [Self; 2] = [Self::Dmy, Self::Mdy];
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Default, Serialize, Deserialize,
)]
pub enum TimeFormat {
    H12,
    #[default]
    H24,
}
impl LocaleString for TimeFormat {
    fn to_format_string(&self) -> String {
        match self {
            TimeFormat::H12 => "%I:%M:%S %p".to_string(),
            TimeFormat::H24 => "%T".to_string(),
        }
    }
}
impl std::fmt::Display for TimeFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            TimeFormat::H12 => "12-hour",
            TimeFormat::H24 => "24-hour",
        };
        f.write_str(repr)
    }
}

impl TimeFormat {
    const VALUES: [Self; 2] = [Self::H12, Self::H24];
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Customization {
    date_format: DateFormat,
    time_format: TimeFormat,
}

impl Customization {
    fn datetime_format(&self) -> String {
        format!(
            "{} {}",
            self.date_format.to_format_string(),
            self.time_format.to_format_string()
        )
    }

    pub fn format_date(&self, date: &NaiveDate) -> String {
        date.format(&self.date_format.to_format_string())
            .to_string()
    }

    pub fn format_datetime(
        &self,
        datetime: &Option<DateTime<Local>>,
    ) -> String {
        if let Some(date) = datetime {
            date.format(&self.datetime_format()).to_string()
        } else {
            "".to_string()
        }
    }

    pub fn parse_datetime(
        &self,
        text: &str,
    ) -> Result<Option<DateTime<Local>>, String> {
        if text.is_empty() {
            return Ok(None);
        }
        let naive =
            NaiveDateTime::parse_from_str(text, &self.datetime_format())
                .map_err(|e| e.to_string())?;
        Ok(Some(Local.from_local_datetime(&naive).unwrap()))
    }
}

#[derive(Clone, Debug)]
pub enum CustomizationMessage {
    SelectTimeFormat(TimeFormat),
    SelectDateFormat(DateFormat),
    Discarded,
    Save,
}

impl Customization {
    pub fn update(
        &mut self,
        message: CustomizationMessage,
    ) -> Command<CustomizationMessage> {
        match message {
            CustomizationMessage::SelectTimeFormat(fmt) => {
                self.time_format = fmt;
                Command::done(CustomizationMessage::Save)
            }
            CustomizationMessage::SelectDateFormat(fmt) => {
                self.date_format = fmt;
                Command::done(CustomizationMessage::Save)
            }
            CustomizationMessage::Discarded | CustomizationMessage::Save => {
                Command::none()
            }
        }
    }

    pub fn view<'a, T: 'a + Clone>(
        &'a self,
        wrapper: &'a impl Fn(CustomizationMessage) -> T,
    ) -> menu::Item<'a, T, iced::Theme, iced::Renderer> {
        menu::Item::with_menu(
            menu_button(
                "Customization",
                wrapper(CustomizationMessage::Discarded),
            )
            .width(iced::Length::Fixed(140f32)),
            menu::Menu::new(vec![
                menu::Item::with_menu(
                    menu_button(
                        "Time format",
                        wrapper(CustomizationMessage::Discarded),
                    ),
                    self.time_format_menu(wrapper),
                ),
                menu::Item::with_menu(
                    menu_button(
                        "Date format",
                        wrapper(CustomizationMessage::Discarded),
                    ),
                    self.date_format_menu(wrapper),
                ),
            ])
            .max_width(120.0),
        )
    }

    fn time_format_menu<'a, T: 'a + Clone>(
        &'a self,
        wrapper: &'a impl Fn(CustomizationMessage) -> T,
    ) -> menu::Menu<'a, T, iced::Theme, iced::Renderer> {
        menu::Menu::new(
            TimeFormat::VALUES
                .iter()
                .map(|f| {
                    menu::Item::new(
                        button(text(f.to_string()))
                            .width(iced::Length::Fill)
                            .on_press_maybe(if self.time_format == *f {
                                None
                            } else {
                                Some(wrapper(
                                    CustomizationMessage::SelectTimeFormat(*f),
                                ))
                            }),
                    )
                })
                .collect(),
        )
        .max_width(120f32)
    }

    fn date_format_menu<'a, T: 'a + Clone>(
        &'a self,
        wrapper: &'a impl Fn(CustomizationMessage) -> T,
    ) -> menu::Menu<'a, T, iced::Theme, iced::Renderer> {
        menu::Menu::new(
            DateFormat::VALUES
                .iter()
                .map(|f| {
                    menu::Item::new(
                        button(text(f.to_string()))
                            .width(iced::Length::Fill)
                            .on_press_maybe(if self.date_format == *f {
                                None
                            } else {
                                Some(wrapper(
                                    CustomizationMessage::SelectDateFormat(*f),
                                ))
                            }),
                    )
                })
                .collect(),
        )
        .max_width(120f32)
    }
}
