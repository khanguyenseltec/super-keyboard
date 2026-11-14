//! The keys of the current layer, and the pointer handling that presses them.
//!
//! A press starts on the key (`pointerdown`); moves, releases and cancels are
//! handled on the whole keyboard, because a finger that swipes along the space
//! bar or slides off a key still belongs to the key it started on.

use dioxus::prelude::*;
use super_keyboard_core::{Action, Key, Keyboard, Latch, PointerId};

use crate::state::AppState;

#[component]
pub fn KeyboardView() -> Element {
    let mut controller = use_context::<AppState>().keyboard;
    let rows: Vec<Vec<KeyFace>> = {
        let keyboard = controller.keyboard.read();
        keyboard
            .layer()
            .rows
            .iter()
            .enumerate()
            .map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .map(|(c, key)| {
                        let pressed = controller.is_pressed(keyboard.key_id(r, c));
                        KeyFace::new(&keyboard, key, pressed)
                    })
                    .collect()
            })
            .collect()
    };

    rsx! {
        div {
            class: "keyboard",
            role: "group",
            aria_label: "Keyboard",
            onpointermove: move |evt| controller.moved(pointer_id(&evt), pointer_x(&evt)),
            onpointerup: move |evt| controller.release(pointer_id(&evt)),
            onpointercancel: move |evt| controller.cancel(pointer_id(&evt)),
            // A mouse dragged off the keyboard never sends `pointerup` here.
            onpointerleave: move |evt| controller.release(pointer_id(&evt)),
            for (r, row) in rows.into_iter().enumerate() {
                div { key: "{r}", class: "row",
                    for (c, face) in row.into_iter().enumerate() {
                        KeyCap { key: "{c}", row: r, col: c, face }
                    }
                }
            }
        }
    }
}

#[component]
fn KeyCap(row: usize, col: usize, face: KeyFace) -> Element {
    let mut controller = use_context::<AppState>().keyboard;
    rsx! {
        button {
            class: "{face.class}",
            style: "flex-grow: {face.width}",
            aria_label: "{face.name}",
            // Keys fire on touch, not on release, and must not steal focus.
            onpointerdown: move |evt| {
                evt.prevent_default();
                controller.press(pointer_id(&evt), row, col, pointer_x(&evt));
            },
            span { class: "label", "{face.label}" }
            if let Some(hint) = &face.hint {
                span { class: "hint", aria_hidden: true, "{hint}" }
            }
        }
    }
}

fn pointer_id(evt: &PointerEvent) -> PointerId {
    evt.pointer_id().into()
}

fn pointer_x(evt: &PointerEvent) -> f32 {
    evt.client_coordinates().x as f32
}

/// How one key looks right now.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyFace {
    pub label: String,
    /// The hold action, drawn small in the corner.
    pub hint: Option<String>,
    /// What screen readers announce.
    pub name: String,
    /// CSS classes; see `assets/main.css`.
    pub class: String,
    pub width: f32,
}

impl KeyFace {
    pub fn new(keyboard: &Keyboard, key: &Key, pressed: bool) -> Self {
        let mut class = String::from("key");
        let mut name = match &key.tap {
            Action::Text(_) => key.label.clone(),
            Action::Key(named) => named.to_string(),
            Action::Modifier(modifier) => modifier.to_string(),
            Action::Layer(layer) => format!("{layer} layer"),
        };
        match &key.tap {
            Action::Modifier(modifier) => {
                class.push_str(" mod");
                match keyboard.latch(*modifier) {
                    Latch::Off => {}
                    Latch::OneShot => {
                        class.push_str(" oneshot");
                        name.push_str(", armed");
                    }
                    Latch::Locked => {
                        class.push_str(" locked");
                        name.push_str(", locked");
                    }
                }
            }
            Action::Layer(layer) => {
                class.push_str(" layer");
                if *layer == keyboard.layer().name {
                    class.push_str(" active");
                    name.push_str(", showing");
                }
            }
            Action::Key(_) => class.push_str(" special"),
            Action::Text(_) if key.tap.single_char().is_none() => class.push_str(" op"),
            Action::Text(_) => {}
        }
        let hint = keyboard.hold_hint(key);
        if let Some(hint) = &hint {
            class.push_str(" has-hint");
            name = format!("{name}, hold for {hint}");
        }
        if key.swipe.is_some() {
            class.push_str(" swipe");
        }
        if pressed {
            class.push_str(" pressed");
        }
        Self {
            label: keyboard.display_label(key).into_owned(),
            hint,
            name,
            class,
            width: key.width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super_keyboard_core::{Keymap, Modifier, NamedKey};

    fn face(keyboard: &Keyboard, tap: &Action, pressed: bool) -> KeyFace {
        let (_, key) = keyboard
            .keymap()
            .keys()
            .find(|(id, key)| id.layer == keyboard.layer_index() && key.tap == *tap)
            .unwrap();
        KeyFace::new(keyboard, key, pressed)
    }

    #[test]
    fn letters_with_hold_actions_show_a_hint() {
        let keyboard = Keyboard::new(Keymap::builtin());
        let q = face(&keyboard, &Action::Text("q".into()), false);
        assert_eq!((q.label.as_str(), q.hint.as_deref()), ("q", Some("1")));
        assert_eq!(q.class, "key has-hint");
        assert_eq!(q.name, "q, hold for 1");
    }

    #[test]
    fn modifiers_show_their_latch() {
        let mut keyboard = Keyboard::new(Keymap::builtin());
        let shift = Action::Modifier(Modifier::Shift);
        assert_eq!(face(&keyboard, &shift, false).class, "key mod");
        keyboard.apply(&shift);
        let armed = face(&keyboard, &shift, true);
        assert_eq!(armed.class, "key mod oneshot pressed");
        assert_eq!(armed.name, "Shift, armed");
        keyboard.apply(&shift);
        assert_eq!(face(&keyboard, &shift, false).class, "key mod locked");
        assert_eq!(face(&keyboard, &Action::Text("a".into()), false).label, "A");
    }

    #[test]
    fn special_operator_layer_and_swipe_keys() {
        let mut keyboard = Keyboard::new(Keymap::builtin());
        let space = face(&keyboard, &Action::Key(NamedKey::Space), false);
        assert_eq!(space.class, "key special swipe");
        assert_eq!(space.width, 3.0);
        let sym = Action::Layer("sym".into());
        assert_eq!(face(&keyboard, &sym, false).class, "key layer");
        keyboard.apply(&sym);
        assert_eq!(face(&keyboard, &sym, false).class, "key layer active");
        assert_eq!(
            face(&keyboard, &Action::Text("->".into()), false).class,
            "key op"
        );
    }
}
