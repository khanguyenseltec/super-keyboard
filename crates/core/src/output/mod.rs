//! What the keyboard sends to the focused application, and where it goes.
//!
//! The engine turns taps into [`Output`] values. An [`OutputSink`] delivers them:
//! the in-app [`PreviewSink`] today, and platform sinks (Android `InputConnection`,
//! Linux uinput / XTest / portal) as they land. See `docs/ARCHITECTURE.md`.

mod buffer;
mod preview;

pub use buffer::{BufferView, Motion, TextBuffer};
pub use preview::PreviewSink;

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

impl Output {
    /// A named key pressed with `mods`.
    pub fn named(key: NamedKey, mods: Modifiers) -> Self {
        Self::Key {
            key: KeyCode::Named(key),
            mods,
        }
    }
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
///
/// This is the whole platform boundary: a backend that can insert text and press
/// keys with modifiers can host Super Keyboard. Sinks that cannot send a key
/// should ignore it rather than panic.
pub trait OutputSink {
    fn emit(&mut self, output: Output);
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
        assert_eq!(
            Output::named(NamedKey::Left, Modifiers::NONE.with(Modifier::Shift)).to_string(),
            "Shift+Left"
        );
    }
}
