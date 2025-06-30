use iced::keyboard::key::Named as NamedKey;
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{keyboard, Element, Fill, Task as Command};
use log::error;

use crate::state::State;
use crate::widgets::{close_button, icon_text, link, CustomWidget};

#[derive(Clone, Copy, Debug)]
pub struct LegalInfo;

#[derive(Clone, Debug)]
pub enum LegalInfoMessage {
    Close,
    OpenLink(String),
}

impl CustomWidget<LegalInfoMessage> for LegalInfo {
    fn view(&self, _state: &State) -> iced::Element<'_, LegalInfoMessage> {
        let content = column![
            close_button(LegalInfoMessage::Close),
            column![
                text("This client is MIT-licensed, you can find the full text of the license by following the link below:"),
                link("License", "https://github.com/sterliakov/toggl/blob/master/LICENSE".to_string(), LegalInfoMessage::OpenLink),
            ].spacing(4),
            column![
                text("We do not share any information with third parties other than Toggl itself. By using this application you agree to comply with Toggl legal requirements:"),
                bullets([
                    bullet_link("Privacy Policy", "https://toggl.com/track/legal/privacy/"),
                    bullet_link("Terms of Service", "https://toggl.com/track/legal/terms/"),
                ]),
            ].spacing(4),
        ]
        .spacing(12);

        scrollable(container(content).center_x(Fill).padding(10)).into()
    }

    fn update(
        &mut self,
        message: LegalInfoMessage,
        _state: &State,
    ) -> Command<LegalInfoMessage> {
        use LegalInfoMessage::*;
        if let OpenLink(ref url) = message {
            // TODO: open_browser would be better but refuses to work with FF on my PC
            if let Err(e) = opener::open(url) {
                error!("Failed to open browser: {e:?}");
            }
        }
        Command::none()
    }

    fn handle_key(
        &mut self,
        key: NamedKey,
        modifiers: keyboard::Modifiers,
    ) -> Option<Command<LegalInfoMessage>> {
        if matches!(key, NamedKey::Escape) && modifiers.is_empty() {
            Some(Command::done(LegalInfoMessage::Close))
        } else {
            None
        }
    }
}

impl LegalInfo {
    pub const fn new() -> Self {
        Self {}
    }
}

fn bullets<'a, T: 'a + Clone>(
    items: impl IntoIterator<Item = impl Into<Element<'a, T>>>,
) -> Column<'a, T> {
    Column::with_children(items.into_iter().map(|item| {
        row![icon_text(iced_fonts::Bootstrap::Dot), item.into()]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(8)
            .into()
    }))
}

fn bullet_link<'a>(
    name: &'a str,
    url: &'a str,
) -> button::Button<'a, LegalInfoMessage> {
    link(name, url.to_string(), LegalInfoMessage::OpenLink).padding(
        iced::Padding {
            top: 2.0,
            ..iced::Padding::default()
        },
    )
}
