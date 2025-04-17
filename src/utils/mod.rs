use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone,
};
use iced::keyboard::Modifiers;

mod client;

pub use client::{Client, Result as NetResult};

pub fn duration_to_hms(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
    format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
}

pub fn duration_to_hm(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
    format!("{}:{:0>2}", hours, minutes)
}

pub fn to_start_of_week(date: DateTime<Local>) -> DateTime<Local> {
    // TODO: start of week is configurable in toggl API, use it
    let mon_naive = NaiveDate::from_isoywd_opt(
        date.year(),
        date.iso_week().week(),
        chrono::Weekday::Mon,
    )
    .unwrap();
    Local
        .from_local_datetime(&NaiveDateTime::new(mon_naive, NaiveTime::MIN))
        .unwrap()
}

pub trait ExactModifiers {
    /// Is exactly one modifier that is Ctrl or Cmd pressed?
    fn is_exact_ctrl_or_cmd(&self) -> bool;
    /// Is exactly one modifier pressed?
    fn is_exact(&self) -> bool;
}

impl ExactModifiers for Modifiers {
    fn is_exact(&self) -> bool {
        self.bits().count_ones() == 1
    }

    fn is_exact_ctrl_or_cmd(&self) -> bool {
        (self.control() || self.macos_command()) && self.is_exact()
    }
}

#[cfg(test)]
mod test {
    use chrono::{Local, TimeDelta};

    use super::*;

    #[test]
    fn test_start_of_week() {
        let cases = [
            (
                Local.with_ymd_and_hms(2025, 4, 17, 10, 11, 12).unwrap(),
                "2025-04-14T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2022, 3, 2, 10, 11, 12).unwrap(),
                "2022-02-28T00:00:00",
            ),
        ];
        for (d, res) in cases {
            let offset = d.offset().local_minus_utc();
            let tz_suffix = duration_to_hm(&TimeDelta::seconds(offset.into()));
            let tz_suffix = format!(
                "{}{:0>5}",
                if offset < 0 { "-" } else { "+" },
                tz_suffix
            );
            assert_eq!(
                to_start_of_week(d).to_rfc3339(),
                format!("{res}{tz_suffix}")
            )
        }
    }
}
