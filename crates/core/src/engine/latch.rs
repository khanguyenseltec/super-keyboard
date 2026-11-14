//! Modifier latches: how tapping a modifier key arms, locks and releases it.

use crate::action::{Modifier, Modifiers};

/// The state of one modifier key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Latch {
    #[default]
    Off,
    /// Applies to the next key, then turns off.
    OneShot,
    /// Applies to every key until tapped again.
    Locked,
}

impl Latch {
    /// The state after one more tap: `Off → OneShot → Locked → Off`.
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::OneShot,
            Self::OneShot => Self::Locked,
            Self::Locked => Self::Off,
        }
    }

    pub fn is_on(self) -> bool {
        self != Self::Off
    }
}

/// One latch per modifier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Latches([Latch; Modifier::ALL.len()]);

impl Latches {
    pub fn get(&self, modifier: Modifier) -> Latch {
        self.0[modifier.index()]
    }

    /// Advances a modifier's latch, as tapping its key does.
    pub fn cycle(&mut self, modifier: Modifier) {
        let latch = &mut self.0[modifier.index()];
        *latch = latch.next();
    }

    /// Modifiers that will apply to the next key.
    pub fn active(&self) -> Modifiers {
        Modifier::ALL
            .into_iter()
            .filter(|m| self.get(*m).is_on())
            .collect()
    }

    /// Returns the active modifiers and releases the one-shot ones.
    pub fn consume(&mut self) -> Modifiers {
        let mods = self.active();
        self.release_one_shots();
        mods
    }

    pub fn release_one_shots(&mut self) {
        for latch in &mut self.0 {
            if *latch == Latch::OneShot {
                *latch = Latch::Off;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_off_one_shot_locked_off() {
        let mut latches = Latches::default();
        assert_eq!(latches.get(Modifier::Alt), Latch::Off);
        latches.cycle(Modifier::Alt);
        assert_eq!(latches.get(Modifier::Alt), Latch::OneShot);
        latches.cycle(Modifier::Alt);
        assert_eq!(latches.get(Modifier::Alt), Latch::Locked);
        latches.cycle(Modifier::Alt);
        assert_eq!(latches.get(Modifier::Alt), Latch::Off);
    }

    #[test]
    fn consume_releases_only_one_shots() {
        let mut latches = Latches::default();
        latches.cycle(Modifier::Ctrl);
        latches.cycle(Modifier::Shift);
        latches.cycle(Modifier::Shift);
        let mods = latches.consume();
        assert!(mods.contains(Modifier::Ctrl) && mods.contains(Modifier::Shift));
        assert_eq!(latches.get(Modifier::Ctrl), Latch::Off);
        assert_eq!(latches.get(Modifier::Shift), Latch::Locked);
    }
}
