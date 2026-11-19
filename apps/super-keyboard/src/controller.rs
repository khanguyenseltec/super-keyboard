//! Connects pointer events to the engine: gestures in, preview output out.
//!
//! The engine decides everything; this module only feeds it events with
//! timestamps, runs a timer while a hold or repeat is pending, and passes the
//! output through the Vietnamese composer to the sink.

use dioxus::{
    core::{Task, spawn_forever},
    prelude::*,
};
use super_keyboard_core::{
    Composer, GestureConfig, Gestures, KeyId, Keyboard, Keymap, OutputSink, PointerId, PressKind,
    PreviewSink, Trigger, VietnameseOptions,
};

use crate::time;

/// The keyboard engine and its preview, shared by the components that draw and
/// press keys. Cheap to copy: every field is a handle.
#[derive(Clone, Copy, PartialEq)]
pub struct KeyboardController {
    pub keyboard: Signal<Keyboard>,
    pub sink: Signal<PreviewSink>,
    /// Telex or VNI typing, between the keyboard and the sink.
    pub vietnamese: Signal<Composer>,
    /// Keys under a finger, for drawing them pressed.
    pressed: Signal<Vec<KeyId>>,
    /// Not reactive: pointer moves must not re-render the keyboard.
    gestures: CopyValue<Gestures>,
    timer: CopyValue<Option<Task>>,
}

impl KeyboardController {
    /// Creates the controller's state in the current scope. Call it from a hook.
    pub fn new(keymap: Keymap, config: GestureConfig, vietnamese: VietnameseOptions) -> Self {
        Self {
            keyboard: Signal::new(Keyboard::new(keymap)),
            sink: Signal::new(PreviewSink::default()),
            vietnamese: Signal::new(Composer::new(vietnamese)),
            pressed: Signal::new(Vec::new()),
            gestures: CopyValue::new(Gestures::new(config)),
            timer: CopyValue::new(None),
        }
    }

    /// A pointer went down on the key at `row`, `col` of the visible layer.
    pub fn press(&mut self, pointer: PointerId, row: usize, col: usize, x: f32) {
        let (id, kind) = {
            let keyboard = self.keyboard.peek();
            let id = keyboard.key_id(row, col);
            let Some(key) = keyboard.key(id) else {
                return;
            };
            (id, PressKind::of(key))
        };
        let triggers = self
            .gestures
            .write()
            .press(pointer, id, kind, x, time::now());
        self.dispatch(triggers);
        self.after_change();
    }

    pub fn moved(&mut self, pointer: PointerId, x: f32) {
        // Most moves are a mouse hovering; skip them without touching any state.
        if !self.gestures.peek().is_tracking(pointer) {
            return;
        }
        let triggers = self.gestures.write().moved(pointer, x);
        self.dispatch(triggers);
    }

    pub fn release(&mut self, pointer: PointerId) {
        if !self.gestures.peek().is_tracking(pointer) {
            return;
        }
        let triggers = self.gestures.write().release(pointer, time::now());
        self.dispatch(triggers);
        self.after_change();
    }

    pub fn cancel(&mut self, pointer: PointerId) {
        if !self.gestures.peek().is_tracking(pointer) {
            return;
        }
        let triggers = self.gestures.write().cancel(pointer);
        self.dispatch(triggers);
        self.after_change();
    }

    /// Whether a finger is on the key `id`.
    pub fn is_pressed(&self, id: KeyId) -> bool {
        self.pressed.read().contains(&id)
    }

    /// Switches to another keymap (another profile, or sections rearranged),
    /// unless it is already showing. Presses in progress are cancelled because
    /// their keys may have moved.
    pub fn set_keymap(&mut self, keymap: Keymap) {
        if *self.keyboard.peek().keymap() == keymap {
            return;
        }
        let triggers = self.gestures.write().cancel_all();
        self.dispatch(triggers);
        self.keyboard.write().set_keymap(keymap);
        self.after_change();
    }

    /// Goes back to the base layer: a way out of any layer, whatever its keys.
    pub fn show_base_layer(&mut self) {
        if self.keyboard.peek().layer_index() != 0 {
            self.keyboard.write().show_base_layer();
        }
    }

    pub fn set_gesture_config(&mut self, config: GestureConfig) {
        if *self.gestures.peek().config() != config {
            self.gestures.write().set_config(config);
            self.reschedule();
        }
    }

    pub fn set_vietnamese(&mut self, options: VietnameseOptions) {
        if self.vietnamese.peek().options() != options {
            self.vietnamese.write().set_options(options);
        }
    }

    pub fn clear_preview(&mut self) {
        self.sink.write().clear();
        self.vietnamese.write().reset();
    }

    fn dispatch(&mut self, triggers: Vec<Trigger>) {
        if triggers.is_empty() {
            return;
        }
        let outputs: Vec<_> = {
            let mut keyboard = self.keyboard.write();
            triggers
                .into_iter()
                .filter_map(|trigger| keyboard.trigger(trigger))
                .collect()
        };
        if !outputs.is_empty() {
            let mut composer = self.vietnamese.write();
            let mut sink = self.sink.write();
            for output in outputs {
                for output in composer.process(output) {
                    sink.emit(output);
                }
            }
        }
    }

    fn after_change(&mut self) {
        let pressed: Vec<KeyId> = self.gestures.peek().pressed_keys().collect();
        if *self.pressed.peek() != pressed {
            self.pressed.set(pressed);
        }
        self.reschedule();
    }

    /// Runs a timer until the next hold or repeat is due, replacing any timer
    /// already running.
    fn reschedule(&mut self) {
        if let Some(task) = self.timer.write().take() {
            task.cancel();
        }
        if self.gestures.peek().next_deadline().is_none() {
            return;
        }
        let mut this = *self;
        // On the root scope, so the timer outlives the key that started it (keys
        // are re-created when the layer changes).
        let task = spawn_forever(async move {
            loop {
                // A statement of its own, so the borrow ends before the writes below.
                let Some(deadline) = this.gestures.peek().next_deadline() else {
                    break;
                };
                time::sleep(deadline.saturating_sub(time::now())).await;
                let triggers = this.gestures.write().tick(time::now());
                this.dispatch(triggers);
            }
            this.timer.set(None);
        });
        self.timer.set(Some(task));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use super_keyboard_core::{Action, Layout, Modifier, NamedKey};

    /// Runs `test` with a controller inside a Dioxus runtime, as event handlers do.
    fn with_controller(test: impl FnOnce(KeyboardController)) {
        fn root() -> Element {
            use_context_provider(|| {
                KeyboardController::new(
                    Keymap::builtin(),
                    GestureConfig::default(),
                    VietnameseOptions::default(),
                )
            });
            rsx! {}
        }
        let mut dom = VirtualDom::new(root);
        dom.rebuild_in_place();
        dom.in_scope(ScopeId::APP, || {
            test(consume_context::<KeyboardController>())
        });
    }

    /// Row and column of the first base-layer key whose tap is `action`.
    fn position(controller: &KeyboardController, action: &Action) -> (usize, usize) {
        let keyboard = controller.keyboard.peek();
        let (id, _) = keyboard
            .keymap()
            .keys()
            .find(|(id, key)| id.layer == keyboard.layer_index() && key.tap == *action)
            .unwrap();
        (id.row, id.col)
    }

    fn tap(controller: &mut KeyboardController, action: &Action) {
        let (row, col) = position(controller, action);
        controller.press(1, row, col, 0.0);
        controller.release(1);
    }

    fn text(controller: &KeyboardController) -> String {
        controller.sink.peek().text().to_owned()
    }

    #[test]
    fn taps_type_into_the_preview_and_show_pressed_keys() {
        with_controller(|mut controller| {
            let (row, col) = position(&controller, &Action::Text("a".into()));
            controller.press(1, row, col, 0.0);
            assert!(controller.is_pressed(controller.keyboard.peek().key_id(row, col)));
            controller.release(1);
            assert!(!controller.is_pressed(controller.keyboard.peek().key_id(row, col)));
            // `q` has a hold action, so its tap fires on a quick release.
            tap(&mut controller, &Action::Text("q".into()));
            assert_eq!(text(&controller), "aq");
        });
    }

    #[test]
    fn holding_a_key_fires_its_hold_action() {
        with_controller(|mut controller| {
            let (row, col) = position(&controller, &Action::Text("q".into()));
            controller.press(1, row, col, 0.0);
            std::thread::sleep(Duration::from_millis(350));
            controller.release(1);
            assert_eq!(text(&controller), "1");
        });
    }

    #[test]
    fn swiping_the_space_bar_moves_the_cursor_and_shift_selects() {
        with_controller(|mut controller| {
            for c in ["a", "b", "c"] {
                tap(&mut controller, &Action::Text(c.into()));
            }
            tap(&mut controller, &Action::Modifier(Modifier::Shift));
            let (row, col) = position(&controller, &Action::Key(NamedKey::Space));
            controller.press(2, row, col, 100.0);
            controller.moved(2, 75.0);
            controller.release(2);
            let sink = controller.sink.peek();
            assert_eq!(sink.text(), "abc", "a swipe types no space");
            assert_eq!(sink.buffer().view().selected, "bc");
        });
    }

    #[test]
    fn telex_turns_taps_into_vietnamese() {
        with_controller(|mut controller| {
            controller.set_vietnamese(VietnameseOptions {
                enabled: true,
                ..VietnameseOptions::default()
            });
            for c in ["v", "i", "e", "e", "t", "j"] {
                tap(&mut controller, &Action::Text(c.into()));
            }
            assert_eq!(text(&controller), "việt");
            assert_eq!(controller.vietnamese.peek().composing(), "việt");
            tap(&mut controller, &Action::Key(NamedKey::Backspace));
            assert_eq!(text(&controller), "việ");
            controller.clear_preview();
            assert_eq!(controller.vietnamese.peek().composing(), "");
        });
    }

    #[test]
    fn layout_changes_cancel_presses() {
        with_controller(|mut controller| {
            let (row, col) = position(&controller, &Action::Text("q".into()));
            controller.press(1, row, col, 0.0);
            let other =
                Layout::from_toml("name = \"x\"\n[[layers]]\nname = \"a\"\nrows = [[\"z\"]]")
                    .unwrap();
            controller.set_keymap(Keymap::from(&other));
            controller.release(1);
            assert_eq!(text(&controller), "", "the cancelled tap does not fire");
            assert_eq!(controller.keyboard.peek().keymap().name, "x");
        });
    }
}
