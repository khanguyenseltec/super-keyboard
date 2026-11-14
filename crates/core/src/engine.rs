//! The keyboard state machine: current layer and modifier latches.

use crate::{
    action::{Action, Modifier, Modifiers, NamedKey},
    layout::{Layer, Layout},
    output::{KeyCode, Output},
};

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
    fn next(self) -> Self {
        match self {
            Self::Off => Self::OneShot,
            Self::OneShot => Self::Locked,
            Self::Locked => Self::Off,
        }
    }
}

/// A keyboard: a layout plus what is currently active on it.
#[derive(Debug, Clone)]
pub struct Keyboard {
    layout: Layout,
    layer: usize,
    latches: [Latch; Modifier::ALL.len()],
}

impl Keyboard {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            layer: 0,
            latches: [Latch::Off; Modifier::ALL.len()],
        }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// The layer currently showing.
    pub fn layer(&self) -> &Layer {
        &self.layout.layers[self.layer]
    }

    pub fn latch(&self, modifier: Modifier) -> Latch {
        self.latches[modifier.index()]
    }

    /// Modifiers that will apply to the next key.
    pub fn active_modifiers(&self) -> Modifiers {
        Modifier::ALL
            .into_iter()
            .filter(|m| self.latch(*m) != Latch::Off)
            .fold(Modifiers::NONE, Modifiers::with)
    }

    /// The label to draw for a key, taking Shift into account.
    pub fn display_label<'a>(&self, key: &'a crate::layout::Key) -> std::borrow::Cow<'a, str> {
        let shifted = self.latch(Modifier::Shift) != Latch::Off;
        match &key.action {
            Action::Text(text) if shifted && text.chars().count() == 1 => {
                text.to_uppercase().into()
            }
            _ => key.label.as_str().into(),
        }
    }

    /// Taps the key at `row`, `col` of the current layer.
    ///
    /// Returns what to send, or `None` when the tap only changed keyboard state
    /// (a modifier or layer key) or the position has no key.
    pub fn tap(&mut self, row: usize, col: usize) -> Option<Output> {
        let action = self.layer().rows.get(row)?.get(col)?.action.clone();
        self.apply(&action)
    }

    /// Applies an action as if its key had been tapped.
    pub fn apply(&mut self, action: &Action) -> Option<Output> {
        match action {
            Action::Modifier(modifier) => {
                let latch = &mut self.latches[modifier.index()];
                *latch = latch.next();
                None
            }
            Action::Layer(name) => {
                let target = self.layout.layer_index(name)?;
                self.layer = if self.layer == target { 0 } else { target };
                None
            }
            Action::Key(key) => {
                let mods = self.consume_modifiers();
                Some(Output::Key {
                    key: KeyCode::Named(*key),
                    mods,
                })
            }
            Action::Text(text) => {
                let mods = self.consume_modifiers();
                Some(text_output(text, mods))
            }
        }
    }

    /// Returns the active modifiers and releases the one-shot ones.
    fn consume_modifiers(&mut self) -> Modifiers {
        let mods = self.active_modifiers();
        for latch in &mut self.latches {
            if *latch == Latch::OneShot {
                *latch = Latch::Off;
            }
        }
        mods
    }
}

fn text_output(text: &str, mods: Modifiers) -> Output {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        // Multi-character text such as `->` is inserted as written.
        return Output::Text(text.to_owned());
    };
    if c == ' ' {
        return Output::Key {
            key: KeyCode::Named(NamedKey::Space),
            mods,
        };
    }
    if !mods.without(Modifier::Shift).is_empty() {
        return Output::Key {
            key: KeyCode::Char(c),
            mods,
        };
    }
    if mods.contains(Modifier::Shift) {
        Output::Text(c.to_uppercase().collect())
    } else {
        Output::Text(c.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyboard() -> Keyboard {
        Keyboard::new(Layout::builtin())
    }

    fn text(s: &str) -> Action {
        Action::Text(s.to_owned())
    }

    #[test]
    fn one_shot_ctrl_makes_a_chord_then_releases() {
        let mut kb = keyboard();
        assert_eq!(kb.apply(&Action::Modifier(Modifier::Ctrl)), None);
        assert_eq!(kb.latch(Modifier::Ctrl), Latch::OneShot);
        assert_eq!(
            kb.apply(&text("c")),
            Some(Output::Key {
                key: KeyCode::Char('c'),
                mods: Modifiers::NONE.with(Modifier::Ctrl),
            })
        );
        assert_eq!(kb.latch(Modifier::Ctrl), Latch::Off);
        assert_eq!(kb.apply(&text("c")), Some(Output::Text("c".into())));
    }

    #[test]
    fn shift_capitalises_once_and_locks_on_double_tap() {
        let mut kb = keyboard();
        let shift = Action::Modifier(Modifier::Shift);
        kb.apply(&shift);
        assert_eq!(kb.apply(&text("a")), Some(Output::Text("A".into())));
        assert_eq!(kb.apply(&text("a")), Some(Output::Text("a".into())));

        kb.apply(&shift);
        kb.apply(&shift);
        assert_eq!(kb.latch(Modifier::Shift), Latch::Locked);
        assert_eq!(kb.apply(&text("a")), Some(Output::Text("A".into())));
        assert_eq!(kb.apply(&text("b")), Some(Output::Text("B".into())));
        kb.apply(&shift);
        assert_eq!(kb.latch(Modifier::Shift), Latch::Off);
    }

    #[test]
    fn modifiers_combine_and_apply_to_named_keys() {
        let mut kb = keyboard();
        kb.apply(&Action::Modifier(Modifier::Shift));
        assert_eq!(
            kb.apply(&Action::Key(NamedKey::Tab)),
            Some(Output::Key {
                key: KeyCode::Named(NamedKey::Tab),
                mods: Modifiers::NONE.with(Modifier::Shift),
            })
        );
        kb.apply(&Action::Modifier(Modifier::Ctrl));
        kb.apply(&Action::Modifier(Modifier::Shift));
        let out = kb.apply(&text("t")).unwrap();
        assert_eq!(out.to_string(), "Ctrl+Shift+T");
    }

    #[test]
    fn multi_character_text_is_inserted_as_written() {
        let mut kb = keyboard();
        kb.apply(&Action::Modifier(Modifier::Shift));
        assert_eq!(kb.apply(&text("->")), Some(Output::Text("->".into())));
        assert!(kb.active_modifiers().is_empty());
    }

    #[test]
    fn layer_keys_toggle_between_a_layer_and_base() {
        let mut kb = keyboard();
        let sym = Action::Layer("sym".into());
        assert_eq!(kb.apply(&sym), None);
        assert_eq!(kb.layer().name, "sym");
        kb.apply(&sym);
        assert_eq!(kb.layer().name, "base");
    }

    #[test]
    fn tap_uses_the_current_layer_and_ignores_missing_keys() {
        let mut kb = keyboard();
        assert_eq!(kb.tap(99, 0), None);
        let (row, col, expected) = kb
            .layer()
            .rows
            .iter()
            .enumerate()
            .find_map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .find_map(|(c, key)| match &key.action {
                        Action::Text(t) if t == "q" => Some((r, c, t.clone())),
                        _ => None,
                    })
            })
            .expect("base layer has a `q` key");
        assert_eq!(kb.tap(row, col), Some(Output::Text(expected)));
    }

    #[test]
    fn display_label_follows_shift() {
        let mut kb = keyboard();
        let key = crate::layout::Key {
            label: "a".into(),
            action: text("a"),
            width: 1.0,
        };
        assert_eq!(kb.display_label(&key), "a");
        kb.apply(&Action::Modifier(Modifier::Shift));
        assert_eq!(kb.display_label(&key), "A");
    }
}
