use chrono::{DateTime, Datelike, Local};
use iced::widget::{button, row, text_input};
use iced::Element;
use iced_aw::date_picker::Date;
use iced_aw::time_picker::Time;
use iced_aw::{DatePicker, TimePicker};
use iced_fonts::Bootstrap;

use crate::components::icon_button;
use crate::customization::Customization;

#[derive(Clone, Debug)]
pub enum DateTimeEditMessage {
    EditText(String),
    OpenTimePicker,
    CloseTimePicker,
    SubmitTime(Time),
    OpenDatePicker,
    CloseDatePicker,
    SubmitDate(Date),
}

#[derive(Clone, Debug)]
pub struct DateTimeWidget {
    input_label: String,
    input_id: String,

    dt: Option<DateTime<Local>>,
    full_text: String,
    show_time_picker: bool,
    show_date_picker: bool,
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
        }
    }

    pub fn view(
        &self,
        customization: &Customization,
    ) -> Element<DateTimeEditMessage> {
        let ref_time = self.dt.unwrap_or_else(Local::now);
        row![
            text_input(&self.input_label, &self.full_text)
                .id(self.input_id.clone())
                .on_input(DateTimeEditMessage::EditText),
            self.date_picker(ref_time),
            self.time_picker(ref_time, customization),
        ]
        .into()
    }

    fn time_picker(
        &self,
        ref_time: DateTime<Local>,
        customization: &Customization,
    ) -> TimePicker<DateTimeEditMessage, iced::Theme> {
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
        );
        if customization.use_24h() {
            timepicker = timepicker.use_24h();
        }
        timepicker
    }

    fn date_picker(
        &self,
        ref_time: DateTime<Local>,
    ) -> DatePicker<DateTimeEditMessage, iced::Theme> {
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

    pub fn update(
        &mut self,
        message: DateTimeEditMessage,
        customization: &Customization,
    ) {
        match message {
            DateTimeEditMessage::EditText(text) => {
                if let Ok(Some(dt)) = customization.parse_datetime(&text) {
                    self.dt = Some(dt);
                }
                self.full_text = text;
            }
            DateTimeEditMessage::OpenTimePicker => {
                self.show_time_picker = true;
            }
            DateTimeEditMessage::CloseTimePicker => {
                self.show_time_picker = false;
            }
            DateTimeEditMessage::SubmitTime(time) => {
                self.dt = Some(
                    self.dt
                        .unwrap_or_else(Local::now)
                        .with_time(time.into())
                        .unwrap(),
                );
                self.full_text = customization.format_datetime(&self.dt);
                self.show_time_picker = false;
            }
            DateTimeEditMessage::OpenDatePicker => {
                self.show_date_picker = true;
            }
            DateTimeEditMessage::CloseDatePicker => {
                self.show_date_picker = false;
            }
            DateTimeEditMessage::SubmitDate(date) => {
                self.dt = Some(
                    self.dt
                        .unwrap_or_else(Local::now)
                        .with_year(date.year)
                        .expect("Invalid date")
                        .with_month(date.month)
                        .expect("Invalid date")
                        .with_day(date.day)
                        .expect("Invalid date"),
                );
                self.full_text = customization.format_datetime(&self.dt);
                self.show_date_picker = false;
            }
        }
    }

    pub fn handle_esc(&mut self) -> bool {
        //! Returns true if Esc press is intended for this component and can be handled
        if self.show_time_picker {
            self.show_time_picker = false;
            return true;
        }
        if self.show_date_picker {
            self.show_date_picker = false;
            return true;
        }
        false
    }

    pub fn get_value(
        &self,
        customization: &Customization,
    ) -> Result<Option<DateTime<Local>>, String> {
        match customization.parse_datetime(&self.full_text) {
            Err(_) => Err(format!("Invalid start date: {}", self.full_text)),
            Ok(None) => Ok(None),
            Ok(Some(_)) => Ok(self.dt),
        }
    }
}
