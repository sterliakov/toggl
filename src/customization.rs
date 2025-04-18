use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use iced::Task as Command;
use iced_aw::menu;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::entities::Preferences;
use crate::utils::{to_start_of_week, Client, NetResult};
use crate::widgets::{menu_select_item, menu_text, top_level_menu_text};

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
    DmyHyphen,
    DmySlash,
    DmyDot,
    MdyHyphen,
    MdySlash,
    YmdHyphen,
}

impl LocaleString for DateFormat {
    fn to_format_string(&self) -> &'static str {
        match self {
            DateFormat::DmyHyphen => "%d-%m-%y",
            DateFormat::MdyHyphen => "%m-%d-%y",
            DateFormat::DmySlash => "%d/%m/%y",
            DateFormat::MdySlash => "%m/%d/%y",
            DateFormat::DmyDot => "%d.%m.%y",
            DateFormat::YmdHyphen => "%y-%m-%d",
        }
    }
}

impl std::fmt::Display for DateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            DateFormat::DmyHyphen => "dd-mm-yyyy",
            DateFormat::MdyHyphen => "mm-dd-yyyy",
            DateFormat::DmySlash => "dd/mm/yyyy",
            DateFormat::MdySlash => "mm/dd/yyyy",
            DateFormat::DmyDot => "dd.mm.yyyy",
            DateFormat::YmdHyphen => "yyyy-mm-dd",
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
            "DD-MM-YYYY" => Self::DmyHyphen,
            "MM-DD-YYYY" => Self::MdyHyphen,
            "DD/MM/YYYY" => Self::DmySlash,
            "MM/DD/YYYY" => Self::MdySlash,
            "DD.MM.YYYY" => Self::DmyDot,
            "YYYY-MM-DD" => Self::YmdHyphen,
            other => {
                warn!("Unknown date format: {other}");
                Self::default()
            }
        }
    }
}

impl DateFormat {
    const VALUES: [Self; 6] = [
        Self::DmySlash,
        Self::DmyHyphen,
        Self::DmyDot,
        Self::MdySlash,
        Self::MdyHyphen,
        Self::YmdHyphen,
    ];
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

impl TimeFormat {
    const VALUES: [Self; 2] = [Self::H12, Self::H24];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeekDay(pub chrono::Weekday);

impl Default for WeekDay {
    fn default() -> Self {
        Self(chrono::Weekday::Mon)
    }
}

impl std::ops::Deref for WeekDay {
    type Target = chrono::Weekday;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TogglConvertible<u8> for WeekDay {
    // Toggl uses Sun = 0 while chrono uses Mon = 1
    fn to_toggl(&self) -> u8 {
        self.0.num_days_from_sunday().try_into().unwrap()
    }
    fn from_toggl(value: &u8) -> Self {
        let off_by_one: chrono::Weekday =
            (*value).try_into().expect("bad start day");
        Self(off_by_one.pred())
    }
}

impl WeekDay {
    const VALUES: [Self; 7] = [
        Self(chrono::Weekday::Mon),
        Self(chrono::Weekday::Tue),
        Self(chrono::Weekday::Wed),
        Self(chrono::Weekday::Thu),
        Self(chrono::Weekday::Fri),
        Self(chrono::Weekday::Sat),
        Self(chrono::Weekday::Sun),
    ];
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Customization {
    date_format: DateFormat,
    time_format: TimeFormat,
    week_start_day: WeekDay,
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
            week_start_day: WeekDay::from_toggl(&value.beginning_of_week),
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

    pub async fn save(self, client: &Client) -> NetResult<()> {
        let prefs: Preferences = self.into();
        prefs.save(client).await
    }
}

#[derive(Clone, Debug)]
pub enum CustomizationMessage {
    SelectTimeFormat(TimeFormat),
    SelectDateFormat(DateFormat),
    SelectWeekBeginning(WeekDay),
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
                menu::Item::with_menu(
                    menu_text(
                        "Week beginning",
                        wrapper(CustomizationMessage::Discarded),
                    ),
                    self.week_beginning_menu(wrapper),
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
}
