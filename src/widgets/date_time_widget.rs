use chrono::{DateTime, Datelike, Local};
use iced::keyboard::key::Named as NamedKey;
use iced::widget::{button, column, row, text, text_input};
use iced::{keyboard, Element, Task as Command};
use iced_aw::date_picker::Date;
use iced_aw::time_picker::Time;
use iced_aw::{DatePicker, TimePicker};
use iced_fonts::Bootstrap;
use log::warn;

use super::{icon_button, CustomWidget};
use crate::customization::Customization;
use crate::state::State;

#[derive(Clone, Debug)]
pub enum DateTimeEditMessage {
    EditText(String),
    OpenTimePicker,
    CloseTimePicker,
    SubmitTime(Time),
    OpenDatePicker,
    CloseDatePicker,
    SubmitDate(Date),
    /// Should be handled externally: Enter pressed (submit)
    Finish,
}

#[derive(Clone, Debug)]
pub struct DateTimeWidget {
    input_label: String,
    input_id: String,

    dt: Option<DateTime<Local>>,
    full_text: String,
    show_time_picker: bool,
    show_date_picker: bool,
    error: Option<String>,
}

impl CustomWidget<DateTimeEditMessage> for DateTimeWidget {
    fn view(&self, state: &State) -> Element<'_, DateTimeEditMessage> {
        let ref_time = self.dt.unwrap_or_else(Local::now);
        column![row![
            text_input(&self.input_label, &self.full_text)
                .id(self.input_id.clone())
                .on_input(DateTimeEditMessage::EditText)
                .on_submit(DateTimeEditMessage::Finish),
            self.date_picker(ref_time),
            self.time_picker(ref_time, &state.customization),
        ]]
        .push_maybe(self.error.clone().map(|e| text(e).style(text::danger)))
        .into()
    }

    fn update(
        &mut self,
        message: DateTimeEditMessage,
        state: &State,
    ) -> Command<DateTimeEditMessage> {
        use DateTimeEditMessage::*;
        let customization = &state.customization;

        match message {
            EditText(text) => {
                if let Ok(Some(dt)) = customization.parse_datetime(&text) {
                    self.dt = Some(dt);
                    self.error = None;
                } else {
                    self.dt = None;
                    self.error = Some("Invalid date".to_string());
                }
                self.full_text = text;
            }
            OpenTimePicker => {
                self.show_time_picker = true;
            }
            CloseTimePicker => {
                self.show_time_picker = false;
            }
            SubmitTime(time) => {
                self.dt = Some(with_time(self.dt, time, Local::now));
                self.full_text = customization.format_datetime(&self.dt);
                self.error = None;
                self.show_time_picker = false;
            }
            OpenDatePicker => {
                self.show_date_picker = true;
            }
            CloseDatePicker => {
                self.show_date_picker = false;
            }
            SubmitDate(date) => {
                self.dt = Some(with_date(self.dt, date, Local::now));
                self.full_text = customization.format_datetime(&self.dt);
                self.error = None;
                self.show_date_picker = false;
            }
            Finish => {}
        }
        Command::none()
    }

    fn handle_key(
        &mut self,
        key: NamedKey,
        _modifiers: keyboard::Modifiers,
    ) -> Option<Command<DateTimeEditMessage>> {
        //! Returns None if key press is not intended for this component
        //! and command to run otherwise.
        match (self.show_time_picker, self.show_date_picker, key) {
            (true, false, NamedKey::Escape) => {
                Some(Command::done(DateTimeEditMessage::CloseTimePicker))
            }
            (false, true, NamedKey::Escape) => {
                Some(Command::done(DateTimeEditMessage::CloseDatePicker))
            }
            _ => None,
        }
    }
}

impl DateTimeWidget {
    pub fn new(
        start: Option<DateTime<Local>>,
        input_label: impl ToString,
        input_id: impl ToString,
        customization: &Customization,
    ) -> Self {
        Self {
            input_label: input_label.to_string(),
            input_id: input_id.to_string(),
            dt: start,
            full_text: customization.format_datetime(&start),
            show_time_picker: false,
            show_date_picker: false,
            error: None,
        }
    }

    fn time_picker(
        &self,
        ref_time: DateTime<Local>,
        customization: &Customization,
    ) -> TimePicker<'_, DateTimeEditMessage, iced::Theme> {
        let but = icon_button(Bootstrap::ClockFill)
            .on_press(DateTimeEditMessage::OpenTimePicker)
            .width(24)
            .style(button::secondary);
        let mut timepicker = TimePicker::new(
            self.show_time_picker,
            ref_time.time(),
            but,
            DateTimeEditMessage::CloseTimePicker,
            DateTimeEditMessage::SubmitTime,
        )
        .show_seconds();
        if customization.use_24h() {
            timepicker = timepicker.use_24h();
        }
        timepicker
    }

    fn date_picker(
        &self,
        ref_time: DateTime<Local>,
    ) -> DatePicker<'_, DateTimeEditMessage, iced::Theme> {
        let but = icon_button(Bootstrap::CalendarDateFill)
            .on_press(DateTimeEditMessage::OpenDatePicker)
            .width(24)
            .style(button::secondary);
        DatePicker::new(
            self.show_date_picker,
            ref_time.date_naive(),
            but,
            DateTimeEditMessage::CloseDatePicker,
            DateTimeEditMessage::SubmitDate,
        )
    }

    pub fn get_value(&self) -> Result<Option<DateTime<Local>>, String> {
        self.error.as_ref().map_or(Ok(self.dt), |e| Err(e.clone()))
    }
}

fn with_time(
    dt: Option<DateTime<Local>>,
    time: Time,
    fallback: impl Fn() -> DateTime<Local>,
) -> DateTime<Local> {
    dt.unwrap_or_else(&fallback)
        .with_time(time.into())
        .single()
        .unwrap_or_else(|| {
            warn!("Ambiguous time, using now as a fallback");
            fallback()
        })
}

fn with_date(
    dt: Option<DateTime<Local>>,
    date: Date,
    fallback: impl Fn() -> DateTime<Local>,
) -> DateTime<Local> {
    dt.unwrap_or_else(&fallback)
        .with_year(date.year)
        .and_then(|d| d.with_month(date.month))
        .and_then(|d| d.with_day(date.day))
        .unwrap_or_else(|| {
            warn!("Ambiguous date, using now as a fallback");
            fallback()
        })
}
