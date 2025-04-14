use chrono::Duration;
use iced::keyboard::Modifiers;

pub fn duration_to_hms(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
    format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
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
