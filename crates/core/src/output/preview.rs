//! The in-app sink behind the preview panel.

use std::collections::VecDeque;

use super::{KeyCode, Motion, Output, OutputSink, TextBuffer};
use crate::action::{Modifier, Modifiers, NamedKey};

/// An in-app sink that edits a local [`TextBuffer`] and records recent output.
///
/// Used by the app's preview panel and by tests; real typing into other apps goes
/// through platform sinks. The buffer understands text, Space, Enter, Tab,
/// Backspace, Delete and cursor keys (Shift selects, Ctrl moves by word); every
/// other chord only appears in the log.
#[derive(Debug, Clone, Default)]
pub struct PreviewSink {
    buffer: TextBuffer,
    log: VecDeque<String>,
}

impl PreviewSink {
    /// How many outputs the log keeps.
    pub const LOG_LIMIT: usize = 32;

    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// The text typed so far.
    pub fn text(&self) -> &str {
        self.buffer.text()
    }

    /// The most recent outputs, oldest first.
    pub fn log(&self) -> impl DoubleEndedIterator<Item = &str> + ExactSizeIterator {
        self.log.iter().map(String::as_str)
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.log.clear();
    }

    fn record(&mut self, output: &Output) {
        if self.log.len() == Self::LOG_LIMIT {
            self.log.pop_front();
        }
        self.log.push_back(output.to_string());
    }
}

/// What a named key does to the preview text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edit {
    Insert(&'static str),
    Backspace,
    Delete,
    Move { motion: Motion, select: bool },
}

impl Edit {
    /// The edit for `key` pressed with `mods`, if the preview has one.
    fn for_key(key: NamedKey, mods: Modifiers) -> Option<Self> {
        // Alt and Super chords are for the application, not for editing text.
        if mods.contains(Modifier::Alt) || mods.contains(Modifier::Super) {
            return None;
        }
        let ctrl = mods.contains(Modifier::Ctrl);
        let select = mods.contains(Modifier::Shift);
        let motion = match key {
            NamedKey::Left if ctrl => Motion::WordLeft,
            NamedKey::Right if ctrl => Motion::WordRight,
            NamedKey::Home if ctrl => Motion::Start,
            NamedKey::End if ctrl => Motion::End,
            NamedKey::Left => Motion::Left,
            NamedKey::Right => Motion::Right,
            NamedKey::Up => Motion::Up,
            NamedKey::Down => Motion::Down,
            NamedKey::Home => Motion::LineStart,
            NamedKey::End => Motion::LineEnd,
            NamedKey::PageUp => Motion::Start,
            NamedKey::PageDown => Motion::End,
            // Other Ctrl chords (Ctrl+Enter, Ctrl+Backspace, …) are left to apps.
            _ if ctrl => return None,
            NamedKey::Space => return Some(Self::Insert(" ")),
            NamedKey::Enter => return Some(Self::Insert("\n")),
            NamedKey::Tab if !select => return Some(Self::Insert("\t")),
            NamedKey::Backspace => return Some(Self::Backspace),
            NamedKey::Delete => return Some(Self::Delete),
            _ => return None,
        };
        Some(Self::Move { motion, select })
    }

    fn apply(self, buffer: &mut TextBuffer) {
        match self {
            Self::Insert(text) => buffer.insert(text),
            Self::Backspace => buffer.backspace(),
            Self::Delete => buffer.delete(),
            Self::Move { motion, select } => buffer.move_cursor(motion, select),
        }
    }
}

impl OutputSink for PreviewSink {
    fn emit(&mut self, output: Output) {
        self.record(&output);
        match output {
            Output::Text(text) => self.buffer.insert(&text),
            Output::Key {
                key: KeyCode::Named(key),
                mods,
            } => {
                if let Some(edit) = Edit::for_key(key, mods) {
                    edit.apply(&mut self.buffer);
                }
            }
            // Character chords such as Ctrl+C only appear in the log.
            Output::Key {
                key: KeyCode::Char(_),
                ..
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(sink: &mut PreviewSink, key: NamedKey, mods: Modifiers) {
        sink.emit(Output::named(key, mods));
    }

    #[test]
    fn edits_text_and_keeps_a_bounded_log() {
        let mut sink = PreviewSink::default();
        sink.emit(Output::Text("fn".into()));
        for k in [NamedKey::Space, NamedKey::Backspace, NamedKey::Enter] {
            key(&mut sink, k, Modifiers::NONE);
        }
        assert_eq!(sink.text(), "fn\n");
        assert_eq!(
            sink.log().collect::<Vec<_>>(),
            [r#""fn""#, "Space", "Backspace", "Enter"]
        );

        for _ in 0..100 {
            sink.emit(Output::Text("x".into()));
        }
        assert_eq!(sink.log().len(), PreviewSink::LOG_LIMIT);
    }

    #[test]
    fn shift_arrows_select_and_ctrl_arrows_jump_words() {
        let mut sink = PreviewSink::default();
        sink.emit(Output::Text("let value".into()));
        let shift = Modifiers::NONE.with(Modifier::Shift);
        let ctrl_shift = shift.with(Modifier::Ctrl);
        key(&mut sink, NamedKey::Left, ctrl_shift);
        assert_eq!(sink.buffer().view().selected, "value");
        sink.emit(Output::Text("x".into()));
        assert_eq!(sink.text(), "let x");
        key(&mut sink, NamedKey::Home, shift);
        key(&mut sink, NamedKey::Delete, Modifiers::NONE);
        assert_eq!(sink.text(), "");
    }

    #[test]
    fn application_chords_do_not_edit() {
        let mut sink = PreviewSink::default();
        sink.emit(Output::Text("ab".into()));
        key(
            &mut sink,
            NamedKey::Backspace,
            Modifiers::NONE.with(Modifier::Alt),
        );
        key(
            &mut sink,
            NamedKey::Tab,
            Modifiers::NONE.with(Modifier::Shift),
        );
        sink.emit(Output::Key {
            key: KeyCode::Char('c'),
            mods: Modifiers::NONE.with(Modifier::Ctrl),
        });
        assert_eq!(sink.text(), "ab");
        assert_eq!(sink.log().count(), 4);
    }

    #[test]
    fn clear_empties_text_and_log() {
        let mut sink = PreviewSink::default();
        sink.emit(Output::Text("x".into()));
        sink.clear();
        assert_eq!(sink.text(), "");
        assert_eq!(sink.log().count(), 0);
    }
}
