//! User configuration for Super Keyboard.
//!
//! Everything a user can change lives in one directory
//! (`~/.config/super-keyboard` on Linux, see [`ConfigDir::detect`]):
//!
//! ```text
//! settings.toml        profile, theme, key timing and size, keyboard size, style,
//!                      profile menu, profiles
//! layouts/*.toml       user layouts; `layouts/dev.toml` replaces the built-in `dev`
//! themes/*.json        user themes; `themes/nord.json` replaces the built-in `nord`
//! ```
//!
//! - [`settings`]: the [`Settings`] model, its file format, and
//!   [`settings::SettingsDocument`] for saving changes without losing comments.
//! - [`style`]: colour, font and size overrides ([`Style`]), validated so a
//!   settings file can only ever set safe CSS values.
//! - [`theme`]: themes as JSON ([`Theme`]), the built-in ones, and the
//!   [`ThemeLibrary`] of built-in and user themes.
//! - [`library`]: built-in and user layouts by id ([`LayoutLibrary`]).
//! - [`profile`]: profiles (a layout plus an arrangement of its sections), the
//!   presets that ship with the app, and the [`ProfileCatalog`] the menu shows.
//! - [`UserConfig`]: loads all of the above, collects [`Problem`]s instead of
//!   failing, and on reload keeps the last good version of anything that is
//!   broken mid-edit.
//! - [`Fingerprint`]: cheap change detection for live reload.
//!
//! The crate has no UI dependency. Front ends (the Dioxus app today, an Android
//! IME host later) share it so settings mean the same thing everywhere.

mod dir;
mod fingerprint;
pub mod library;
pub mod profile;
pub mod settings;
pub mod style;
pub mod theme;
mod user_config;

pub use dir::ConfigDir;
pub use fingerprint::Fingerprint;
pub use library::{LayoutEntry, LayoutId, LayoutLibrary, LayoutOrigin};
pub use profile::{DEFAULT_PROFILE, Profile, ProfileCatalog, ProfileOrigin, ProfileSpec};
pub use settings::{
    Align, KeySettings, KeyboardSettings, Limits, PreviewSettings, Settings, SettingsError,
    ThemeChoice, ThemeSettings,
};
pub use style::{FontRole, Style, StyleValue, StyleVar, ValueKind};
pub use theme::{Appearance, Theme, ThemeEntry, ThemeError, ThemeLibrary, ThemeOrigin};
pub use user_config::{ActiveTheme, Problem, SaveError, UserConfig};
