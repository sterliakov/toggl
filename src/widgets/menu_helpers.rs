use iced::widget::button::Status;
use iced::widget::{button, text};
use iced_aw::menu;

const MENU_TEXT_SIZE: u16 = 11;

pub fn default_button_text<'a>(content: impl ToString) -> text::Text<'a> {
    text(content.to_string()).size(MENU_TEXT_SIZE)
}

fn menu_button_base<'a, T>(
    content: impl Into<iced::Element<'a, T>>,
) -> button::Button<'a, T> {
    button(content).padding([2, 4]).width(iced::Length::Fill)
}

pub fn menu_button<'a, T>(
    content: impl Into<iced::Element<'a, T>>,
    message: Option<T>,
) -> button::Button<'a, T> {
    menu_button_base(content)
        .style(|theme, status| button::Style {
            background: match status {
                Status::Hovered => {
                    Some(theme.extended_palette().primary.weak.color.into())
                }
                _ => None,
            },
            ..button::text(theme, status)
        })
        .on_press_maybe(message)
}

pub fn menu_select_item<'a, T: Clone + 'a>(
    content: impl ToString,
    selected: bool,
    message: T,
) -> menu::Item<'a, T, iced::Theme, iced::Renderer> {
    menu::Item::new(menu_button(
        default_button_text(content),
        if selected { None } else { Some(message) },
    ))
}

pub fn menu_text_disabled<'a, T>(
    content: impl ToString,
) -> button::Button<'a, T> {
    menu_button(default_button_text(content), None)
}

pub fn menu_text<'a, T>(
    content: impl ToString,
    message: T,
) -> button::Button<'a, T> {
    menu_button(default_button_text(content), Some(message))
}

pub fn top_level_menu_text<'a, T>(
    content: impl ToString,
    message: T,
) -> button::Button<'a, T> {
    menu_text(content, message).width(iced::Length::Shrink)
}
