use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Weekday};
use iced::Task as Command;
use iced_aw::menu;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::entities::Preferences;
use crate::utils::{to_start_of_week, Client, NetResult};
use crate::widgets::{
    menu_select_item, menu_text, menu_text_disabled, top_level_menu_text,
};

trait LocaleString {
    fn to_format_string(&self) -> &'static str;
}

trait TogglConvertible<T> {
    fn to_toggl(&self) -> T;
    fn from_toggl(value: &T) -> Self;
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
    fn to_format_string(&self) -> &'static str {
        match self {
            DateFormat::Dmy => "%d-%m-%y",
            DateFormat::Mdy => "%m-%d-%y",
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

impl TogglConvertible<String> for DateFormat {
    fn to_toggl(&self) -> String {
        self.to_string().to_uppercase()
    }
    fn from_toggl(value: &String) -> Self {
        match value as &str {
            "DD-MM-YYYY" => Self::Dmy,
            "MM-DD-YYYY" => Self::Mdy,
            other => {
                warn!("Unknown date format: {other}");
                Self::default()
            }
        }
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
    fn to_format_string(&self) -> &'static str {
        match self {
            TimeFormat::H12 => "%I:%M:%S %p",
            TimeFormat::H24 => "%T",
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

impl TogglConvertible<String> for TimeFormat {
    fn to_toggl(&self) -> String {
        match self {
            Self::H24 => "H:mm".to_string(),
            Self::H12 => "h:mm A".to_string(),
        }
    }
    fn from_toggl(value: &String) -> Self {
        match value as &str {
            "H:mm" => Self::H24,
            "h:mm A" => Self::H12,
            other => {
                warn!("Unknown date format: {other}");
                Self::default()
            }
        }
    }
}

impl TogglConvertible<u8> for Weekday {
    fn to_toggl(&self) -> u8 {
        self.number_from_sunday().try_into().unwrap()
    }
    fn from_toggl(value: &u8) -> Self {
        let off_by_one: Self = (*value).try_into().expect("bad start day");
        off_by_one.pred()
    }
}

impl TimeFormat {
    const VALUES: [Self; 2] = [Self::H12, Self::H24];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Customization {
    date_format: DateFormat,
    time_format: TimeFormat,
    #[serde(default = "default_week_day")]
    week_start_day: chrono::Weekday,
}

const fn default_week_day() -> chrono::Weekday {
    chrono::Weekday::Mon
}

impl Default for Customization {
    fn default() -> Self {
        Self {
            date_format: DateFormat::default(),
            time_format: TimeFormat::default(),
            week_start_day: chrono::Weekday::Mon,
        }
    }
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

impl From<Preferences> for Customization {
    fn from(value: Preferences) -> Self {
        Self {
            date_format: DateFormat::from_toggl(&value.date_format),
            time_format: TimeFormat::from_toggl(&value.time_format),
            week_start_day: Weekday::from_toggl(&value.beginning_of_week),
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
        to_start_of_week(dt, self.week_start_day)
    }

    pub async fn save(self, client: &Client) -> NetResult<()> {
        let prefs: Preferences = self.into();
        prefs.save(client).await
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
                menu::Item::new(menu_text_disabled(format!(
                    "Week starts on {}",
                    self.week_start_day
                ))),
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
}
