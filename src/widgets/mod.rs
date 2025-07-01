use iced::alignment::Horizontal;
use iced::widget::{button, text, Container};
use iced_fonts::bootstrap::icon_to_string;
use iced_fonts::{Bootstrap, BOOTSTRAP_FONT};

mod base;
mod date_time_widget;
mod menu_helpers;
mod running_entry;
mod tag_editor;
mod text_editor_ext;

pub use base::CustomWidget;
pub use date_time_widget::{DateTimeEditMessage, DateTimeWidget};
pub use menu_helpers::{
    default_button_text, menu_button, menu_icon, menu_select_item, menu_text,
    menu_text_disabled, top_level_menu_text,
};
pub use running_entry::{RunningEntry, RunningEntryMessage};
pub use tag_editor::{TagEditor, TagEditorMessage};
pub use text_editor_ext::{TextEditorExt, TextEditorMessage};

pub fn icon_text<'a>(icon: Bootstrap) -> text::Text<'a> {
    text(icon_to_string(icon)).font(BOOTSTRAP_FONT)
}

pub fn icon_button<'a, T>(icon: Bootstrap) -> button::Button<'a, T> {
    button(
        icon_text(icon)
            .align_x(Horizontal::Center)
            .width(iced::Length::Fill),
    )
}

pub fn close_button<'a, T: Clone + 'a>(message: T) -> Container<'a, T> {
    Container::new(
        button(icon_text(Bootstrap::X).size(24).width(iced::Length::Shrink))
            .on_press(message)
            .style(button::text),
    )
    .align_x(Horizontal::Right)
    .width(iced::Length::Fill)
}

pub fn link<'a, T: Clone + 'a>(
    text: &'a str,
    url: String,
    after_open: impl Fn(String) -> T,
) -> button::Button<'a, T> {
    button(text)
        .style(|theme: &iced::Theme, status| button::Style {
            text_color: theme.extended_palette().primary.base.color,
            ..button::text(theme, status)
        })
        .on_press(after_open(url))
        .padding([0, 0])
}
