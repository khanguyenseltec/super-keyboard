//! Platform-independent engine for Super Keyboard.
//!
//! - [`layout`]: layouts made of sections, loaded from TOML ("keyboard as code");
//!   [`Arrangement`]s that switch sections on and off and reorder them; and the
//!   [`Keymap`] that results.
//! - [`action`]: what a key does, and the `<name>` notation used in layout files.
//! - [`gesture`]: when a key fires: tap, long-press hold, repeat and swipe, as a
//!   clock-free state machine.
//! - [`engine`]: the [`Keyboard`] state machine (current layer, one-shot and locked
//!   modifiers) that turns gesture [`Trigger`]s into [`Output`].
//! - [`output`]: [`Output`] values, the [`OutputSink`] trait that platform
//!   backends implement, and the in-app [`PreviewSink`].
//!
//! A front end wires them together:
//!
//! ```
//! use std::time::Duration;
//! use super_keyboard_core::{
//!     Gestures, Keyboard, Keymap, Layout, OutputSink, PressKind, PreviewSink,
//! };
//!
//! let mut keyboard = Keyboard::new(Keymap::from(&Layout::builtin()));
//! let mut gestures = Gestures::default();
//! let mut sink = PreviewSink::default();
//!
//! // A finger goes down on row 2, column 0 of the base layer (`a`) and lifts.
//! // (The number row and terminal keys are off by default.)
//! let key = keyboard.key_id(2, 0);
//! let kind = PressKind::of(keyboard.key(key).unwrap());
//! let mut triggers = gestures.press(1, key, kind, 0.0, Duration::ZERO);
//! triggers.extend(gestures.release(1, Duration::from_millis(80)));
//! for trigger in triggers {
//!     if let Some(output) = keyboard.trigger(trigger) {
//!         sink.emit(output);
//!     }
//! }
//! assert_eq!(sink.text(), "a");
//! ```
//!
//! This crate has no UI or platform dependencies so it can be tested on any host
//! and shared by the desktop app, the Android IME and future front ends.

pub mod action;
pub mod engine;
pub mod gesture;
pub mod layout;
pub mod output;

pub use action::{Action, Modifier, Modifiers, NamedKey, ParseActionError};
pub use engine::{Keyboard, Latch};
pub use gesture::{Direction, GestureConfig, Gestures, PointerId, PressKind, Trigger};
pub use layout::{
    Arrangement, Block, Key, KeyId, KeyProblem, Keymap, Layer, LayerDef, Layout, LayoutError,
    Section, Swipe,
};
pub use output::{BufferView, KeyCode, Motion, Output, OutputSink, PreviewSink, TextBuffer};
