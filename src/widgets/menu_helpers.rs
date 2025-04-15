use iced::widget::button::Status;
use iced::widget::{button, text};
use iced_aw::menu;

fn menu_button<'a, T>(
    content: impl ToString,
) -> button::Button<'a, T, iced::Theme, iced::Renderer> {
    button(text(content.to_string()).size(11))
        .padding([2, 4])
        .width(iced::Length::Fill)
}

pub fn menu_select_item<'a, T: Clone + 'a>(
    content: impl ToString,
    selected: bool,
    message: T,
) -> menu::Item<'a, T, iced::Theme, iced::Renderer> {
    menu::Item::new(menu_button(content).on_press_maybe(if selected {
        None
    } else {
        Some(message)
    }))
}

pub fn menu_text_disabled<'a, T>(
    content: impl ToString,
) -> button::Button<'a, T, iced::Theme, iced::Renderer> {
    menu_button(content).style(|theme, status| button::Style {
        background: match status {
            Status::Hovered => {
                Some(theme.extended_palette().primary.weak.color.into())
            }
            _ => None,
        },
        ..button::Style::default()
    })
}

pub fn menu_text<'a, T>(
    content: impl ToString,
    message: T,
) -> button::Button<'a, T, iced::Theme, iced::Renderer> {
    menu_text_disabled(content).on_press(message)
}

pub fn top_level_menu_text<'a, T>(
    content: impl ToString,
    message: T,
) -> button::Button<'a, T, iced::Theme, iced::Renderer> {
    menu_text(content, message).width(iced::Length::Shrink)
}
