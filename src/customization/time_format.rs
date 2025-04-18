use log::warn;
use serde::{Deserialize, Serialize};

use super::{LocaleString, TogglConvertible};

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
            Self::H12 => "%I:%M:%S %p",
            Self::H24 => "%T",
        }
    }
}

impl std::fmt::Display for TimeFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Self::H12 => "12-hour",
            Self::H24 => "24-hour",
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
    pub const VALUES: [Self; 2] = [Self::H12, Self::H24];
}
