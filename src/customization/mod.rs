use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use iced::Task as Command;
use iced_aw::menu;
use serde::{Deserialize, Serialize};

use crate::entities::{Preferences, WorkspaceId};
use crate::utils::{to_start_of_week, Client, NetResult};
use crate::widgets::{
    default_button_text, menu_button, menu_select_item, menu_text,
    top_level_menu_text,
};

mod date_format;
mod time_format;
mod weekday;

pub use date_format::DateFormat;
pub use time_format::TimeFormat;
pub use weekday::WeekDay;

trait LocaleString {
    fn to_format_string(&self) -> &'static str;
}

trait TogglConvertible<T> {
    fn to_toggl(&self) -> T;
    fn from_toggl(value: &T) -> Self;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Customization {
    date_format: DateFormat,
    time_format: TimeFormat,

    #[cfg(not(test))]
    week_start_day: WeekDay,
    #[cfg(test)]
    pub week_start_day: WeekDay,

    #[serde(default)]
    pub dark_mode: bool,
}

impl From<Customization> for Preferences {
    fn from(value: Customization) -> Self {
        Preferences {
            date_format: value.date_format.to_toggl(),
            time_format: value.time_format.to_toggl(),
            beginning_of_week: value.week_start_day.to_toggl(),
        }
    }
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
        date.format(self.date_format.to_format_string()).to_string()
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

    pub fn use_24h(&self) -> bool {
        match self.time_format {
            TimeFormat::H12 => false,
            TimeFormat::H24 => true,
        }
    }

    pub fn to_start_of_week(&self, dt: DateTime<Local>) -> DateTime<Local> {
        to_start_of_week(dt, *self.week_start_day)
    }

    pub async fn save(
        self,
        default_workspace_id: Option<WorkspaceId>,
        client: &Client,
    ) -> NetResult<()> {
        let prefs: Preferences = self.into();
        prefs.save(default_workspace_id, client).await
    }
}

#[derive(Clone, Debug)]
pub enum CustomizationMessage {
    SelectTimeFormat(TimeFormat),
    SelectDateFormat(DateFormat),
    SelectWeekBeginning(WeekDay),
    ToggleDarkMode,
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
            CustomizationMessage::SelectWeekBeginning(day) => {
                self.week_start_day = day;
                Command::done(CustomizationMessage::Save)
            }
            CustomizationMessage::ToggleDarkMode => {
                self.dark_mode = !self.dark_mode;
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
        use iced::alignment::Vertical;
        use iced::widget::{row, toggler};

        menu::Item::with_menu(
            top_level_menu_text(
                "Customization",
                wrapper(CustomizationMessage::Discarded),
            ),
            menu::Menu::new(vec![
                menu::Item::with_menu(
                    menu_text(
                        "Time format",
                        wrapper(CustomizationMessage::Discarded),
                    ),
                    self.time_format_menu(wrapper),
                ),
                menu::Item::with_menu(
                    menu_text(
                        "Date format",
                        wrapper(CustomizationMessage::Discarded),
                    ),
                    self.date_format_menu(wrapper),
                ),
                menu::Item::with_menu(
                    menu_text(
                        "Week beginning",
                        wrapper(CustomizationMessage::Discarded),
                    ),
                    self.week_beginning_menu(wrapper),
                ),
                menu::Item::new(menu_button(
                    row![
                        default_button_text("Dark mode")
                            .width(iced::Length::Fill),
                        toggler(self.dark_mode),
                    ]
                    .align_y(Vertical::Center),
                    Some(wrapper(CustomizationMessage::ToggleDarkMode)),
                )),
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
                    menu_select_item(
                        f,
                        self.time_format == *f,
                        wrapper(CustomizationMessage::SelectTimeFormat(*f)),
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
                    menu_select_item(
                        f,
                        self.date_format == *f,
                        wrapper(CustomizationMessage::SelectDateFormat(*f)),
                    )
                })
                .collect(),
        )
        .max_width(120f32)
    }

    fn week_beginning_menu<'a, T: 'a + Clone>(
        &'a self,
        wrapper: &'a impl Fn(CustomizationMessage) -> T,
    ) -> menu::Menu<'a, T, iced::Theme, iced::Renderer> {
        menu::Menu::new(
            WeekDay::VALUES
                .iter()
                .map(|f| {
                    menu_select_item(
                        **f,
                        self.week_start_day == *f,
                        wrapper(CustomizationMessage::SelectWeekBeginning(*f)),
                    )
                })
                .collect(),
        )
        .max_width(120f32)
    }

    pub fn update_from_preferences(self, preferences: Preferences) -> Self {
        Self {
            date_format: DateFormat::from_toggl(&preferences.date_format),
            time_format: TimeFormat::from_toggl(&preferences.time_format),
            week_start_day: WeekDay::from_toggl(&preferences.beginning_of_week),
            ..self
        }
    }
}
