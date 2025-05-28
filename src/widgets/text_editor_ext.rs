use iced::advanced::text::highlighter::PlainText;
use iced::keyboard;
use iced::keyboard::key::Named as NamedKey;
use iced::widget::text_editor;
use iced::widget::text_editor::{Action, Binding, Content, Motion, TextEditor};

use crate::utils::ExactModifiers;

#[derive(Debug)]
pub struct TextEditorExt {
    content: Content,
    history: Vec<EditorHistory>,
    original_text: String,
}

type CursorPosition = (usize, usize);

#[derive(Clone, Debug)]
struct EditorHistory {
    action: Action,
    cursor_position: CursorPosition,
}

impl TextEditorExt {
    pub fn new(text: &Option<impl ToString>) -> Self {
        let original_text = text
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or("".to_string());
        Self {
            content: Content::with_text(&original_text),
            history: vec![],
            original_text,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TextEditorMessage {
    Original(Action),
    Undo,
}

impl TextEditorExt {
    pub fn view<'a>(&'a self) -> TextEditor<'a, PlainText, TextEditorMessage> {
        text_editor(&self.content)
            .key_binding(|press| {
                if !matches!(press.status, text_editor::Status::Focused) {
                    return None;
                }
                match press.key.as_ref() {
                    keyboard::Key::Named(NamedKey::Backspace)
                    | keyboard::Key::Character("w")
                        if press.modifiers.is_exact_ctrl_or_cmd() =>
                    {
                        Some(Binding::Sequence(vec![
                            Binding::Select(Motion::WordLeft),
                            Binding::Delete,
                        ]))
                    }
                    keyboard::Key::Named(NamedKey::Delete)
                        if press.modifiers.is_empty() =>
                    {
                        Some(Binding::Delete)
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
                    keyboard::Key::Character("z")
                        if press.modifiers.is_exact_ctrl_or_cmd() =>
                    {
                        Some(Binding::Custom(TextEditorMessage::Undo))
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
            .on_action(TextEditorMessage::Original)
    }

    pub fn update(&mut self, action: TextEditorMessage) {
        use TextEditorMessage::*;

        match action {
            Undo => {
                let mut step_to_undo: EditorHistory;
                loop {
                    let Some(entry) = self.history.pop() else {
                        return;
                    };
                    if entry.action.is_edit() {
                        step_to_undo = entry;
                        break;
                    }
                }
                loop {
                    let Some(entry) = self.history.last() else {
                        break;
                    };
                    if entry.action.is_edit() {
                        break;
                    }
                    step_to_undo = self.history.pop().unwrap();
                }

                let mut content = Content::with_text(&self.original_text);
                // Preserve selections - they affect next edit
                let mut move_before_next_edit = true;
                for EditorHistory {
                    action,
                    cursor_position,
                } in self.history.clone()
                {
                    if !action.is_edit() || move_before_next_edit {
                        move_cursor_to(&mut content, cursor_position);
                    }
                    move_before_next_edit = action.is_edit();
                    content.perform(action);
                }
                move_cursor_to(&mut content, step_to_undo.cursor_position);
                self.content = content;
            }
            Original(action) => {
                if is_important(&action) {
                    // Subsequent selections are represented as range selections,
                    // but still keep the old ones to restore cursor position
                    // correctly.
                    self.history.push(EditorHistory {
                        action: action.clone(),
                        cursor_position: self.content.cursor_position(),
                    });
                }
                self.content.perform(action);
            }
        }
    }

    pub fn get_value(&self) -> String {
        self.content.text()
    }
}

fn is_select(action: &Action) -> bool {
    matches!(
        action,
        Action::Select(_)
            | Action::SelectWord
            | Action::SelectLine
            | Action::SelectAll
    )
}

fn is_important(action: &Action) -> bool {
    //! Should this action be stored in history?
    is_select(action) || action.is_edit()
}

fn move_cursor_to(content: &mut Content, (line, col): CursorPosition) {
    content.perform(Action::Move(Motion::DocumentStart));
    for _ in 0..line {
        content.perform(Action::Move(Motion::Down));
    }
    for _ in 0..col {
        content.perform(Action::Move(Motion::Right));
    }
}
