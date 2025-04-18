use serde::{Deserialize, Serialize};

use super::TogglConvertible;

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
    pub const VALUES: [Self; 7] = [
        Self(chrono::Weekday::Mon),
        Self(chrono::Weekday::Tue),
        Self(chrono::Weekday::Wed),
        Self(chrono::Weekday::Thu),
        Self(chrono::Weekday::Fri),
        Self(chrono::Weekday::Sat),
        Self(chrono::Weekday::Sun),
    ];
}

#[cfg(test)]
mod test {
    use super::{TogglConvertible, WeekDay};

    #[test]
    fn test_conversion() {
        use chrono::Weekday;

        let pairs = [
            (Weekday::Mon, 1),
            (Weekday::Tue, 2),
            (Weekday::Wed, 3),
            (Weekday::Thu, 4),
            (Weekday::Fri, 5),
            (Weekday::Sat, 6),
            (Weekday::Sun, 0),
        ];
        for (day, toggl) in pairs {
            assert_eq!(WeekDay::from_toggl(&toggl), WeekDay(day));
            assert_eq!(WeekDay(day).to_toggl(), toggl);
        }
    }
}
