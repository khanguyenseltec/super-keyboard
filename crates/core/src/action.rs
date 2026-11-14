//! Key actions and the `<name>` notation used in layout files.
//!
//! A key in a layout file is written as a short string:
//!
//! | Written as      | Meaning                                                   |
//! |-----------------|-----------------------------------------------------------|
//! | `a`, `->`, `::` | Insert that text.                                         |
//! | `<esc>`         | Press a named key (see [`NamedKey::from_name`]).          |
//! | `<ctrl>`        | Arm a modifier for the next key (see [`Modifier`]).       |
//! | `<layer:sym>`   | Switch to the layer named `sym`.                          |
//!
//! Anything that is not `<identifier>` is plain text, so operators such as `<=>`
//! or `</` need no escaping. Text that looks like a key name (for example `<div>`)
//! is written with the table form `{ text = "<div>" }` instead.

use std::fmt;

/// What happens when a key is tapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Insert text. A single character responds to Shift and forms a chord with
    /// Ctrl, Alt or Super; longer text is inserted as written.
    Text(String),
    /// Press a key that does not insert text, such as `<esc>` or `<left>`.
    Key(NamedKey),
    /// Arm a modifier for the next key. Tapping it again locks it; a third tap
    /// releases it.
    Modifier(Modifier),
    /// Switch to the named layer, or back to the first layer if that layer is
    /// already showing.
    Layer(String),
}

impl Action {
    /// Parses the short notation used for keys in layout files.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let Some(name) = bracketed_name(spec) else {
            if spec.is_empty() {
                return Err("empty key".to_owned());
            }
            return Ok(Self::Text(spec.to_owned()));
        };
        if let Some(layer) = name.strip_prefix("layer:") {
            if layer.is_empty() {
                return Err("`<layer:>` needs a layer name".to_owned());
            }
            return Ok(Self::Layer(layer.to_owned()));
        }
        if let Some(modifier) = Modifier::from_name(name) {
            return Ok(Self::Modifier(modifier));
        }
        if let Some(key) = NamedKey::from_name(name) {
            return Ok(Self::Key(key));
        }
        Err(format!(
            "unknown key `<{name}>` (write {{ text = \"<{name}>\" }} to insert it as text)"
        ))
    }

    /// The label shown on a key cap when the layout does not set one.
    pub fn default_label(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Key(key) => key.label(),
            Self::Modifier(modifier) => modifier.label().to_owned(),
            Self::Layer(layer) => layer.clone(),
        }
    }
}

/// Returns the inner name of `<name>` when `spec` uses the key notation.
fn bracketed_name(spec: &str) -> Option<&str> {
    let inner = spec.strip_prefix('<')?.strip_suffix('>')?;
    let is_name = !inner.is_empty()
        && inner
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_'));
    is_name.then_some(inner)
}

/// Keys that do something other than insert text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Escape,
    Tab,
    Enter,
    Backspace,
    Delete,
    Insert,
    Space,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// Function keys `F1` to `F24`.
    F(u8),
}

impl NamedKey {
    /// Looks up a key by the name used inside `<…>`.
    ///
    /// Names: `esc`, `tab`, `enter` (`cr`), `bksp` (`bs`), `del`, `ins`, `space`,
    /// `up`, `down`, `left`, `right`, `home`, `end`, `pgup`, `pgdn`, `f1`–`f24`.
    pub fn from_name(name: &str) -> Option<Self> {
        let key = match name {
            "esc" | "escape" => Self::Escape,
            "tab" => Self::Tab,
            "enter" | "cr" | "return" => Self::Enter,
            "bksp" | "bs" | "backspace" => Self::Backspace,
            "del" | "delete" => Self::Delete,
            "ins" | "insert" => Self::Insert,
            "space" => Self::Space,
            "up" => Self::Up,
            "down" => Self::Down,
            "left" => Self::Left,
            "right" => Self::Right,
            "home" => Self::Home,
            "end" => Self::End,
            "pgup" | "pageup" => Self::PageUp,
            "pgdn" | "pagedown" => Self::PageDown,
            _ => {
                let n: u8 = name.strip_prefix('f')?.parse().ok()?;
                return (1..=24).contains(&n).then_some(Self::F(n));
            }
        };
        Some(key)
    }

    /// Short label for a key cap.
    pub fn label(self) -> String {
        match self {
            Self::Escape => "esc".into(),
            Self::Tab => "tab".into(),
            Self::Enter => "⏎".into(),
            Self::Backspace => "⌫".into(),
            Self::Delete => "del".into(),
            Self::Insert => "ins".into(),
            Self::Space => "space".into(),
            Self::Up => "↑".into(),
            Self::Down => "↓".into(),
            Self::Left => "←".into(),
            Self::Right => "→".into(),
            Self::Home => "home".into(),
            Self::End => "end".into(),
            Self::PageUp => "pgup".into(),
            Self::PageDown => "pgdn".into(),
            Self::F(n) => format!("F{n}"),
        }
    }
}

impl fmt::Display for NamedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Escape => f.write_str("Esc"),
            Self::Tab => f.write_str("Tab"),
            Self::Enter => f.write_str("Enter"),
            Self::Backspace => f.write_str("Backspace"),
            Self::Delete => f.write_str("Delete"),
            Self::Insert => f.write_str("Insert"),
            Self::Space => f.write_str("Space"),
            Self::Up => f.write_str("Up"),
            Self::Down => f.write_str("Down"),
            Self::Left => f.write_str("Left"),
            Self::Right => f.write_str("Right"),
            Self::Home => f.write_str("Home"),
            Self::End => f.write_str("End"),
            Self::PageUp => f.write_str("PageUp"),
            Self::PageDown => f.write_str("PageDown"),
            Self::F(n) => write!(f, "F{n}"),
        }
    }
}

/// Modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    /// The Super / Windows / Command key.
    Super,
}

impl Modifier {
    /// All modifiers, in the order they are written in chords (`Ctrl+Alt+Shift+Super+…`).
    pub const ALL: [Self; 4] = [Self::Ctrl, Self::Alt, Self::Shift, Self::Super];

    /// Looks up a modifier by the name used inside `<…>`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ctrl" | "control" => Some(Self::Ctrl),
            "alt" => Some(Self::Alt),
            "shift" => Some(Self::Shift),
            "super" | "win" | "cmd" => Some(Self::Super),
            _ => None,
        }
    }

    /// Short label for a key cap.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ctrl => "ctrl",
            Self::Alt => "alt",
            Self::Shift => "⇧",
            Self::Super => "super",
        }
    }

    pub(crate) fn index(self) -> usize {
        self as usize
    }

    fn bit(self) -> u8 {
        1 << self.index()
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Shift => "Shift",
            Self::Super => "Super",
        })
    }
}

/// A set of modifiers held together with a key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);

    pub fn contains(self, modifier: Modifier) -> bool {
        self.0 & modifier.bit() != 0
    }

    pub fn insert(&mut self, modifier: Modifier) {
        self.0 |= modifier.bit();
    }

    #[must_use]
    pub fn with(mut self, modifier: Modifier) -> Self {
        self.insert(modifier);
        self
    }

    #[must_use]
    pub fn without(mut self, modifier: Modifier) -> Self {
        self.0 &= !modifier.bit();
        self
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        Modifier::ALL.into_iter().filter(move |m| self.contains(*m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_and_operators_are_text() {
        for spec in ["a", "->", "::", "<", ">", "<>", "<=>", "</", "<=", "{}"] {
            assert_eq!(
                Action::parse(spec),
                Ok(Action::Text(spec.to_owned())),
                "{spec}"
            );
        }
    }

    #[test]
    fn bracketed_names_are_keys_modifiers_and_layers() {
        assert_eq!(Action::parse("<esc>"), Ok(Action::Key(NamedKey::Escape)));
        assert_eq!(Action::parse("<f12>"), Ok(Action::Key(NamedKey::F(12))));
        assert_eq!(
            Action::parse("<ctrl>"),
            Ok(Action::Modifier(Modifier::Ctrl))
        );
        assert_eq!(
            Action::parse("<layer:sym>"),
            Ok(Action::Layer("sym".to_owned()))
        );
    }

    #[test]
    fn unknown_names_and_empty_keys_are_errors() {
        assert!(Action::parse("<div>").is_err());
        assert!(Action::parse("<f25>").is_err());
        assert!(Action::parse("<layer:>").is_err());
        assert!(Action::parse("").is_err());
    }

    #[test]
    fn modifier_sets() {
        let mods = Modifiers::NONE.with(Modifier::Ctrl).with(Modifier::Shift);
        assert!(mods.contains(Modifier::Ctrl) && mods.contains(Modifier::Shift));
        assert!(!mods.contains(Modifier::Alt));
        assert_eq!(
            mods.iter().collect::<Vec<_>>(),
            [Modifier::Ctrl, Modifier::Shift]
        );
        assert!(
            mods.without(Modifier::Ctrl)
                .without(Modifier::Shift)
                .is_empty()
        );
    }
}
