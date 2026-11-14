//! Modifier keys and sets of them.

use std::fmt;

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

    /// Position in [`Modifier::ALL`].
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

    /// Whether only Shift (or nothing) is held: such keys still type text.
    pub fn is_shift_only(self) -> bool {
        self.without(Modifier::Shift).is_empty()
    }

    /// The modifiers in this set, in chord order.
    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        Modifier::ALL.into_iter().filter(move |m| self.contains(*m))
    }
}

impl FromIterator<Modifier> for Modifiers {
    fn from_iter<I: IntoIterator<Item = Modifier>>(iter: I) -> Self {
        iter.into_iter().fold(Self::NONE, Self::with)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn shift_only() {
        assert!(Modifiers::NONE.is_shift_only());
        assert!(Modifiers::NONE.with(Modifier::Shift).is_shift_only());
        assert!(!Modifiers::NONE.with(Modifier::Alt).is_shift_only());
    }

    #[test]
    fn collects_from_an_iterator() {
        let mods: Modifiers = [Modifier::Super, Modifier::Ctrl].into_iter().collect();
        assert_eq!(
            mods,
            Modifiers::NONE.with(Modifier::Ctrl).with(Modifier::Super)
        );
    }
}
