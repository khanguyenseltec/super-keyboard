//! How text combines with held modifiers.

use crate::{
    action::{Modifier, Modifiers, NamedKey},
    output::{KeyCode, Output},
};

/// What typing `text` with `mods` held sends.
///
/// - Multi-character text such as `->` is inserted as written; modifiers do not
///   apply to it.
/// - A single character with Ctrl, Alt or Super becomes a chord (`Ctrl+C`).
/// - A single character with Shift alone is inserted in upper case.
/// - A space is the Space key, so `Shift+Space` and `Ctrl+Space` reach apps.
pub(super) fn text_output(text: &str, mods: Modifiers) -> Output {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return Output::Text(text.to_owned());
    };
    if c == ' ' {
        return Output::named(NamedKey::Space, mods);
    }
    if !mods.is_shift_only() {
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

    const NONE: Modifiers = Modifiers::NONE;

    #[test]
    fn text_and_modifiers() {
        let shift = NONE.with(Modifier::Shift);
        let ctrl = NONE.with(Modifier::Ctrl);
        assert_eq!(text_output("a", NONE), Output::Text("a".into()));
        assert_eq!(text_output("a", shift), Output::Text("A".into()));
        assert_eq!(text_output("ß", shift), Output::Text("SS".into()));
        assert_eq!(text_output("->", ctrl), Output::Text("->".into()));
        assert_eq!(
            text_output("c", ctrl),
            Output::Key {
                key: KeyCode::Char('c'),
                mods: ctrl
            }
        );
        assert_eq!(
            text_output(" ", shift),
            Output::named(NamedKey::Space, shift)
        );
    }
}
