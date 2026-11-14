//! Platform-independent engine for Super Keyboard.
//!
//! - [`layout`]: layouts, layers and keys, loaded from TOML ("keyboard as code").
//! - [`action`]: what a key does, and the `<name>` notation used in layout files.
//! - [`engine`]: the [`Keyboard`] state machine (current layer, one-shot and locked
//!   modifiers) that turns taps into [`Output`].
//! - [`output`]: [`Output`] values and the [`OutputSink`] trait that platform
//!   backends implement.
//!
//! This crate has no UI or platform dependencies so it can be tested on any host
//! and shared by the desktop app, the Android IME and future front ends.

pub mod action;
pub mod engine;
pub mod layout;
pub mod output;

pub use action::{Action, Modifier, Modifiers, NamedKey};
pub use engine::{Keyboard, Latch};
pub use layout::{Key, Layer, Layout, LayoutError};
pub use output::{KeyCode, Output, OutputSink, PreviewSink};
