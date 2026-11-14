//! Super Keyboard app: the Dioxus front end shared by Ubuntu desktop and Android.
//!
//! - [`app`]: the root component.
//! - [`state`]: app-wide state in context: configuration, keyboard, open panel.
//! - [`controller`]: feeds pointer events to the engine and output to the preview.
//! - [`components`]: the top bar, notices, preview, settings panel and keys.
//! - [`style`]: the active theme and user style overrides as CSS.
//! - [`watch`]: live reload of the config directory.
//! - [`time`]: the clock and timer gestures run on.
//!
//! Today the app types into its own preview panel. Platform sinks that type into
//! other applications are tracked in `docs/ROADMAP.md`.

mod app;
mod components;
mod controller;
mod state;
mod style;
mod time;
mod watch;

// `LaunchBuilder::new` is only deprecated in builds without a renderer, such as
// `cargo test --no-default-features`.
#[cfg_attr(
    not(any(feature = "desktop", feature = "mobile", feature = "web")),
    allow(deprecated)
)]
fn main() {
    dioxus::LaunchBuilder::new()
        .with_context(state::AppEnv::detect())
        .launch(app::App);
}
