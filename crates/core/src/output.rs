//! What the keyboard sends to the focused application, and where it goes.
//!
//! The engine turns taps into [`Output`] values. An [`OutputSink`] delivers them:
//! the in-app [`PreviewSink`] today, and platform sinks (Android `InputConnection`,
//! Linux uinput / XTest / portal) as they land. See `docs/ARCHITECTURE.md`.

use std::fmt;

use crate::action::{Modifiers, NamedKey};

/// A key that is sent with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Named(NamedKey),
    /// A character key, as in `Ctrl+C`.
    Char(char),
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(key) => key.fmt(f),
            Self::Char(c) => write!(f, "{}", c.to_uppercase()),
        }
    }
}

/// One thing the keyboard sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// Insert text.
    Text(String),
    /// Press a key, with modifiers held.
    Key { key: KeyCode, mods: Modifiers },
}

impl fmt::Display for Output {
    /// `"->"` for text, `Ctrl+Shift+C` for keys.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "{text:?}"),
            Self::Key { key, mods } => {
                for modifier in mods.iter() {
                    write!(f, "{modifier}+")?;
                }
                key.fmt(f)
            }
        }
    }
}

/// Delivers output to wherever the keyboard is typing.
pub trait OutputSink {
    fn emit(&mut self, output: Output);
}

/// An in-app sink that edits a local text buffer and records every output.
///
/// Used by the app's preview panel and by tests; real typing into other apps
/// goes through platform sinks.
#[derive(Debug, Clone, Default)]
pub struct PreviewSink {
    text: String,
    log: Vec<String>,
}

impl PreviewSink {
    const LOG_LIMIT: usize = 32;

    /// The text typed so far.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The most recent outputs, oldest first.
    pub fn log(&self) -> &[String] {
        &self.log
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.log.clear();
    }
}

impl OutputSink for PreviewSink {
    fn emit(&mut self, output: Output) {
        if self.log.len() == Self::LOG_LIMIT {
            self.log.remove(0);
        }
        self.log.push(output.to_string());

        match output {
            Output::Text(text) => self.text.push_str(&text),
            Output::Key {
                key: KeyCode::Named(key),
                mods,
            } if mods.is_empty() => match key {
                NamedKey::Space => self.text.push(' '),
                NamedKey::Enter => self.text.push('\n'),
                NamedKey::Tab => self.text.push('\t'),
                NamedKey::Backspace => {
                    self.text.pop();
                }
                // Cursor keys and the rest only appear in the log for now.
                _ => {}
            },
            // Chords such as Ctrl+C only appear in the log.
            Output::Key { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Modifier;

    #[test]
    fn displays_text_and_chords() {
        assert_eq!(Output::Text("->".into()).to_string(), r#""->""#);
        let chord = Output::Key {
            key: KeyCode::Char('c'),
            mods: Modifiers::NONE.with(Modifier::Shift).with(Modifier::Ctrl),
        };
        assert_eq!(chord.to_string(), "Ctrl+Shift+C");
    }

    #[test]
    fn preview_edits_text_and_keeps_a_bounded_log() {
        let mut sink = PreviewSink::default();
        sink.emit(Output::Text("fn".into()));
        for key in [NamedKey::Space, NamedKey::Backspace, NamedKey::Enter] {
            sink.emit(Output::Key {
                key: KeyCode::Named(key),
                mods: Modifiers::NONE,
            });
        }
        assert_eq!(sink.text(), "fn\n");
        assert_eq!(sink.log(), [r#""fn""#, "Space", "Backspace", "Enter"]);

        for _ in 0..100 {
            sink.emit(Output::Text("x".into()));
        }
        assert_eq!(sink.log().len(), PreviewSink::LOG_LIMIT);
    }
}
