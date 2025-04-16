use iced::advanced::text::highlighter::PlainText;
use iced::keyboard;
use iced::keyboard::key::Named as NamedKey;
use iced::widget::text_editor;
use iced::widget::text_editor::{Action, Binding, Content, Motion, TextEditor};

use crate::utils::ExactModifiers;

#[derive(Debug)]
pub struct TextEditorExt {
    content: Content,
}

impl TextEditorExt {
    pub fn new(text: &Option<impl ToString>) -> Self {
        Self {
            content: Content::with_text(
                &text
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or("".to_string()),
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextEditorMessage(Action);

impl TextEditorExt {
    pub fn view<'a>(&'a self) -> TextEditor<'a, PlainText, TextEditorMessage> {
        text_editor(&self.content)
            .key_binding(|press| {
                match press.key.as_ref() {
                    keyboard::Key::Named(NamedKey::Backspace)
                    | keyboard::Key::Character("w")
                        if press.modifiers.is_exact_ctrl_or_cmd() =>
                    {
                        Some(Binding::Sequence(vec![
                            Binding::SelectWord,
                            Binding::Backspace,
                            Binding::Backspace, // Preceding whitespace if any
                        ]))
                    }
                    keyboard::Key::Named(NamedKey::Delete)
                        if press.modifiers.is_exact_ctrl_or_cmd() =>
                    {
                        Some(Binding::Sequence(vec![
                            Binding::Select(Motion::WordRight),
                            Binding::Delete,
                        ]))
                    }
                    keyboard::Key::Character("e")
                        if press.modifiers.is_exact_ctrl_or_cmd() =>
                    {
                        Some(Binding::Move(Motion::DocumentEnd))
                    }
                    // Propagate Ctrl+Enter up
                    keyboard::Key::Named(NamedKey::Enter)
                        if press.modifiers.is_exact_ctrl_or_cmd() =>
                    {
                        None
                    }
                    _ => Binding::from_key_press(press),
                }
            })
            .on_action(TextEditorMessage)
    }

    pub fn update(&mut self, action: TextEditorMessage) {
        self.content.perform(action.0);
    }

    pub fn get_value(&self) -> String {
        self.content.text()
    }
}
