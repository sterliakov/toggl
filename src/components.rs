use iced::widget::button;

pub fn menu_button<T>(
    content: &str,
    message: T,
) -> button::Button<'_, T, iced::Theme, iced::Renderer> {
    button(content)
        .style(|_, _| button::Style {
            background: None,
            ..button::Style::default()
        })
        .padding([4, 4])
        .on_press(message)
        .width(iced::Length::Fill)
}
