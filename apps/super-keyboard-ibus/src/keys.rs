//! What the engine does with one key from the physical keyboard, without any
//! D-Bus: the rules here are the whole behaviour, and are tested on their own.
//!
//! The syllable being typed is shown as pre-edit text (underlined, in the
//! application) and committed when it ends: at a space, punctuation, Enter, an
//! arrow, a shortcut, or when the application moves the cursor. Keys the engine
//! does not use are left to the application.

use super_keyboard_core::{Composer, VietnameseOptions};

/// X keysyms the engine looks at.
pub mod keysym {
    pub const BACKSPACE: u32 = 0xff08;
    #[cfg(test)]
    pub const RETURN: u32 = 0xff0d;
    #[cfg(test)]
    pub const LEFT: u32 = 0xff51;
    pub const SHIFT_L: u32 = 0xffe1;
    pub const HYPER_R: u32 = 0xffee;
    pub const ISO_LEVEL3_SHIFT: u32 = 0xfe03;
    pub const ISO_LEVEL5_LOCK: u32 = 0xfe13;
}

/// IBus modifier state bits.
pub mod mask {
    #[cfg(test)]
    pub const SHIFT: u32 = 1 << 0;
    pub const CONTROL: u32 = 1 << 2;
    pub const MOD1: u32 = 1 << 3;
    pub const MOD4: u32 = 1 << 6;
    pub const SUPER: u32 = 1 << 26;
    pub const HYPER: u32 = 1 << 27;
    pub const META: u32 = 1 << 28;
    pub const RELEASE: u32 = 1 << 30;
    /// Modifiers that make a key a shortcut rather than typing.
    pub const SHORTCUT: u32 = CONTROL | MOD1 | MOD4 | SUPER | HYPER | META;
}

/// What the engine tells IBus after a key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    /// The key was used up; the application does not see it.
    pub handled: bool,
    /// Text to commit to the application, before it sees the key.
    pub commit: Option<String>,
}

/// The engine's typing state.
#[derive(Debug, Clone, Default)]
pub struct KeyHandler {
    composer: Composer,
}

impl KeyHandler {
    pub fn new(options: VietnameseOptions) -> Self {
        Self {
            composer: Composer::new(options),
        }
    }

    pub fn options(&self) -> VietnameseOptions {
        self.composer.options()
    }

    /// Changes the options. Returns the syllable in progress, which the caller
    /// commits, so switching method or turning Vietnamese off loses nothing.
    pub fn set_options(&mut self, options: VietnameseOptions) -> Option<String> {
        if options == self.composer.options() {
            return None;
        }
        let pending = self.composer.take();
        self.composer.set_options(options);
        (!pending.is_empty()).then_some(pending)
    }

    /// The syllable being typed, shown as pre-edit text.
    pub fn preedit(&self) -> String {
        self.composer.composing()
    }

    /// Forgets the syllable without committing it. IBus commits pre-edit text
    /// itself when focus leaves or the application resets the input.
    pub fn reset(&mut self) {
        self.composer.reset();
    }

    /// Handles one key event from IBus.
    pub fn key(&mut self, keyval: u32, state: u32) -> Response {
        let pass = Response::default();
        if state & mask::RELEASE != 0 || !self.options().enabled || is_modifier(keyval) {
            return pass;
        }
        if state & mask::SHORTCUT != 0 {
            return self.finish(None, false);
        }
        if keyval == keysym::BACKSPACE {
            return Response {
                handled: self.composer.backspace(),
                commit: None,
            };
        }
        match printable(keyval) {
            Some(c) if self.composer.push_key(c) => Response {
                handled: true,
                commit: None,
            },
            // Space and punctuation end the syllable; the engine types them
            // with it so they arrive in order.
            Some(c) if !self.preedit().is_empty() => self.finish(Some(c), true),
            Some(_) => pass,
            // Enter, Tab, arrows and the rest: finish, then let the
            // application have the key.
            None => self.finish(None, false),
        }
    }

    fn finish(&mut self, key: Option<char>, handled: bool) -> Response {
        let mut text = self.composer.take();
        text.extend(key);
        Response {
            handled,
            commit: (!text.is_empty()).then_some(text),
        }
    }
}

/// The ASCII character a keysym types, if any.
fn printable(keyval: u32) -> Option<char> {
    (0x20..=0x7e)
        .contains(&keyval)
        .then(|| char::from_u32(keyval))
        .flatten()
}

/// Shift, Control, Alt, Super, Caps Lock, AltGr and friends on their own.
fn is_modifier(keyval: u32) -> bool {
    (keysym::SHIFT_L..=keysym::HYPER_R).contains(&keyval)
        || (keysym::ISO_LEVEL3_SHIFT..=keysym::ISO_LEVEL5_LOCK).contains(&keyval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super_keyboard_core::InputMethod;

    fn on() -> KeyHandler {
        KeyHandler::new(VietnameseOptions {
            enabled: true,
            ..VietnameseOptions::default()
        })
    }

    /// Types `keys` and returns what was committed, plus the pre-edit text
    /// still showing.
    fn type_keys(handler: &mut KeyHandler, keys: &str) -> (String, String) {
        let mut committed = String::new();
        for c in keys.chars() {
            let response = handler.key(c as u32, 0);
            committed.extend(response.commit);
            // A key the engine passes on is typed by the application.
            if !response.handled {
                committed.push(c);
            }
        }
        (committed, handler.preedit())
    }

    #[test]
    fn letters_compose_and_a_space_commits_with_the_syllable() {
        let mut h = on();
        assert_eq!(type_keys(&mut h, "Vieej"), (String::new(), "Việ".into()));
        let r = h.key('t' as u32, 0);
        assert_eq!((r.handled, h.preedit().as_str()), (true, "Việt"));
        let r = h.key(' ' as u32, 0);
        assert_eq!(
            r,
            Response {
                handled: true,
                commit: Some("Việt ".into())
            }
        );
        assert_eq!(h.preedit(), "");
        assert_eq!(
            type_keys(&mut h, "Nam, xin chaof."),
            ("Nam, xin chào.".into(), String::new())
        );
    }

    #[test]
    fn a_space_between_words_goes_to_the_application() {
        let mut h = on();
        assert_eq!(h.key(' ' as u32, 0), Response::default());
        assert_eq!(h.key('.' as u32, 0), Response::default());
    }

    #[test]
    fn backspace_edits_the_syllable_then_belongs_to_the_application() {
        let mut h = on();
        type_keys(&mut h, "hoanf");
        assert_eq!(h.preedit(), "hoàn");
        assert!(h.key(keysym::BACKSPACE, 0).handled);
        assert_eq!(h.preedit(), "hòa");
        for _ in 0..3 {
            assert!(h.key(keysym::BACKSPACE, 0).handled);
        }
        assert_eq!(h.preedit(), "");
        assert!(!h.key(keysym::BACKSPACE, 0).handled);
    }

    #[test]
    fn enter_arrows_and_shortcuts_commit_then_pass() {
        let mut h = on();
        type_keys(&mut h, "dduowcj");
        assert_eq!(
            h.key(keysym::RETURN, 0),
            Response {
                handled: false,
                commit: Some("được".into())
            }
        );
        type_keys(&mut h, "as");
        assert_eq!(h.key(keysym::LEFT, 0).commit.as_deref(), Some("á"));
        type_keys(&mut h, "as");
        // Ctrl+S saves: the syllable is committed first, the shortcut passes.
        assert_eq!(
            h.key('s' as u32, mask::CONTROL),
            Response {
                handled: false,
                commit: Some("á".into())
            }
        );
    }

    #[test]
    fn modifier_keys_and_releases_change_nothing() {
        let mut h = on();
        type_keys(&mut h, "VIE");
        assert_eq!(h.key(keysym::SHIFT_L, mask::SHIFT), Response::default());
        assert_eq!(h.key('e' as u32, mask::RELEASE), Response::default());
        assert!(h.key('E' as u32, mask::SHIFT).handled);
        assert_eq!(h.preedit(), "VIÊ");
    }

    #[test]
    fn off_passes_every_key() {
        let mut h = KeyHandler::default();
        assert_eq!(h.key('a' as u32, 0), Response::default());
        assert_eq!(h.preedit(), "");
    }

    #[test]
    fn changing_options_hands_back_the_syllable() {
        let mut h = on();
        type_keys(&mut h, "chaof");
        let vni = VietnameseOptions {
            method: InputMethod::Vni,
            ..h.options()
        };
        assert_eq!(h.set_options(vni).as_deref(), Some("chào"));
        assert_eq!(h.set_options(vni), None);
        assert_eq!(type_keys(&mut h, "Vie65t ").0, "Việt ");
    }

    #[test]
    fn vni_digits_without_a_syllable_type_digits() {
        let mut h = KeyHandler::new(VietnameseOptions {
            enabled: true,
            method: InputMethod::Vni,
            ..VietnameseOptions::default()
        });
        assert_eq!(h.key('2' as u32, 0), Response::default());
        type_keys(&mut h, "x");
        assert_eq!(
            h.key('2' as u32, 0),
            Response {
                handled: true,
                commit: Some("x2".into())
            }
        );
    }

    #[test]
    fn viqr_punctuation_marks_a_syllable_or_ends_it() {
        let mut h = KeyHandler::new(VietnameseOptions {
            enabled: true,
            method: InputMethod::Viqr,
            ..VietnameseOptions::default()
        });
        // With nothing to mark, punctuation is the application's.
        assert_eq!(h.key('.' as u32, 0), Response::default());
        assert_eq!(type_keys(&mut h, "Vie^.t ").0, "Việt ");
        // A mark the syllable cannot take ends it, and is typed after it.
        type_keys(&mut h, "x");
        assert_eq!(
            h.key('?' as u32, 0),
            Response {
                handled: true,
                commit: Some("x?".into())
            }
        );
    }
}
