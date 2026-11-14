//! Keys that do something other than insert text.

use std::fmt;

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

    /// Whether holding this key repeats it unless the layout says otherwise.
    pub fn repeats_by_default(self) -> bool {
        matches!(
            self,
            Self::Backspace
                | Self::Delete
                | Self::Up
                | Self::Down
                | Self::Left
                | Self::Right
                | Self::PageUp
                | Self::PageDown
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_aliases() {
        assert_eq!(NamedKey::from_name("esc"), Some(NamedKey::Escape));
        assert_eq!(NamedKey::from_name("cr"), Some(NamedKey::Enter));
        assert_eq!(NamedKey::from_name("bs"), Some(NamedKey::Backspace));
        assert_eq!(NamedKey::from_name("f1"), Some(NamedKey::F(1)));
        assert_eq!(NamedKey::from_name("f24"), Some(NamedKey::F(24)));
        assert_eq!(NamedKey::from_name("f0"), None);
        assert_eq!(NamedKey::from_name("f25"), None);
        assert_eq!(NamedKey::from_name("nope"), None);
    }

    #[test]
    fn labels_and_display() {
        assert_eq!(NamedKey::Backspace.label(), "⌫");
        assert_eq!(NamedKey::F(5).label(), "F5");
        assert_eq!(NamedKey::PageDown.to_string(), "PageDown");
    }
}
