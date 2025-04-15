use iced::alignment::Horizontal;
use iced::widget::{button, text};
use iced_fonts::bootstrap::icon_to_string;
use iced_fonts::{Bootstrap, BOOTSTRAP_FONT};

mod date_time_widget;
mod menu_helpers;

pub use date_time_widget::{DateTimeEditMessage, DateTimeWidget};
pub use menu_helpers::{
    menu_select_item, menu_text, menu_text_disabled, top_level_menu_text,
};

pub fn icon_text<'a>(
    icon: Bootstrap,
) -> text::Text<'a, iced::Theme, iced::Renderer> {
    text(icon_to_string(icon)).font(BOOTSTRAP_FONT)
}

pub fn icon_button<'a, T>(
    icon: Bootstrap,
) -> button::Button<'a, T, iced::Theme, iced::Renderer> {
    button(
        icon_text(icon)
            .align_x(Horizontal::Center)
            .width(iced::Length::Fill),
    )
}
