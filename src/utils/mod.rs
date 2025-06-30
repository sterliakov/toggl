use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime,
    TimeDelta, TimeZone, Weekday,
};
use iced::keyboard::Modifiers;

mod client;
mod serde;

pub use client::{Client, Result as NetResult};
pub use serde::maybe_vec_deserialize;

pub fn duration_to_hms(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
    format!("{hours}:{minutes:0>2}:{seconds:0>2}")
}

pub fn duration_to_hm(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
    format!("{hours}:{minutes:0>2}")
}

pub fn to_start_of_week(
    date: DateTime<Local>,
    begin_day: Weekday,
) -> DateTime<Local> {
    let mon_naive = NaiveDate::from_isoywd_opt(
        date.year(),
        date.iso_week().week(),
        Weekday::Mon,
    )
    .unwrap();
    let monday = Local
        .from_local_datetime(&NaiveDateTime::new(mon_naive, NaiveTime::MIN))
        .unwrap();
    let offset: i64 = begin_day.num_days_from_monday().into();
    if date.weekday().num_days_from_monday() >= begin_day.num_days_from_monday()
    {
        monday + TimeDelta::days(offset)
    } else {
        monday - TimeDelta::days(7i64 - offset)
    }
}

pub trait ExactModifiers {
    /// Is exactly one modifier that is Ctrl or Cmd pressed?
    fn is_exact_ctrl_or_cmd(&self) -> bool;
    /// Is exactly one modifier pressed?
    fn is_exact(&self) -> bool;
}

impl ExactModifiers for Modifiers {
    fn is_exact(&self) -> bool {
        self.bits().is_power_of_two()
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
                Weekday::Mon,
                "2025-04-14T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2025, 4, 14, 10, 11, 12).unwrap(),
                Weekday::Mon,
                "2025-04-14T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2022, 3, 2, 10, 11, 12).unwrap(),
                Weekday::Mon,
                "2022-02-28T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2025, 4, 17, 10, 11, 12).unwrap(),
                Weekday::Sun,
                "2025-04-13T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2025, 4, 13, 10, 11, 12).unwrap(),
                Weekday::Sun,
                "2025-04-13T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2025, 4, 12, 10, 11, 12).unwrap(),
                Weekday::Sun,
                "2025-04-06T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2025, 4, 17, 10, 11, 12).unwrap(),
                Weekday::Wed,
                "2025-04-16T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2025, 4, 16, 10, 11, 12).unwrap(),
                Weekday::Wed,
                "2025-04-16T00:00:00",
            ),
            (
                Local.with_ymd_and_hms(2025, 4, 15, 10, 11, 12).unwrap(),
                Weekday::Wed,
                "2025-04-09T00:00:00",
            ),
        ];
        for (d, day, res) in cases {
            let offset = d.offset().local_minus_utc();
            let tz_suffix = duration_to_hm(&TimeDelta::seconds(offset.into()));
            let tz_suffix = format!(
                "{}{:0>5}",
                if offset < 0 { "-" } else { "+" },
                tz_suffix
            );
            assert_eq!(
                to_start_of_week(d, day).to_rfc3339(),
                format!("{res}{tz_suffix}")
            );
        }
    }
}
