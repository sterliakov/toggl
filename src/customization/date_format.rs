use log::warn;
use serde::{Deserialize, Serialize};

use super::{LocaleString, TogglConvertible};

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
            Self::DmyHyphen => "%d-%m-%y",
            Self::MdyHyphen => "%m-%d-%y",
            Self::DmySlash => "%d/%m/%y",
            Self::MdySlash => "%m/%d/%y",
            Self::DmyDot => "%d.%m.%y",
            Self::YmdHyphen => "%y-%m-%d",
        }
    }
}

impl std::fmt::Display for DateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Self::DmyHyphen => "dd-mm-yyyy",
            Self::MdyHyphen => "mm-dd-yyyy",
            Self::DmySlash => "dd/mm/yyyy",
            Self::MdySlash => "mm/dd/yyyy",
            Self::DmyDot => "dd.mm.yyyy",
            Self::YmdHyphen => "yyyy-mm-dd",
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
    pub const VALUES: [Self; 6] = [
        Self::DmySlash,
        Self::DmyHyphen,
        Self::DmyDot,
        Self::MdySlash,
        Self::MdyHyphen,
        Self::YmdHyphen,
    ];
}
