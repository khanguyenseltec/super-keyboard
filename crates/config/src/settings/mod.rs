//! `settings.toml`: the profile, theme, key timing and size, style, the profile
//! menu and the user's profiles.
//!
//! ```toml
//! profile = "coding"    # a preset, one of [[profiles]], or a layout's file name
//! theme = "system"      # "system", "light" or "dark"
//!
//! [keys]
//! hold-delay = 300      # milliseconds before a long-press fires
//! repeat-delay = 400    # milliseconds before Backspace, Delete and arrows repeat
//! repeat-rate = 30      # repeats per second
//! swipe-step = 12       # pixels of drag per cursor step on the space bar
//! height = 52           # key height in pixels; leave out to fit the screen
//!
//! [preview]
//! show-log = true
//!
//! [style.dark]
//! accent = "#ff9e64"
//!
//! [menu]                # the profile menu: order, and entries hidden
//! order = ["mine", "coding"]
//! shown = { compact = false }
//!
//! [[profiles]]          # see crate::profile
//! name = "mine"
//! layout = "dev"
//! sections = { number-row = true }
//! ```
//!
//! Every field is optional. Unknown fields and out-of-range numbers are errors,
//! so typos are reported instead of silently ignored.

mod document;
mod error;
mod schema;

pub use document::SettingsDocument;
pub use error::SettingsError;

use std::{fmt, time::Duration};

use super_keyboard_core::{Arrangement, GestureConfig};

use crate::{profile::ProfileSpec, style::Style};

/// Everything `settings.toml` can set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    /// The profile to use, by name. `None` is the default profile.
    pub profile: Option<String>,
    pub theme: ThemeMode,
    pub keys: KeySettings,
    pub preview: PreviewSettings,
    pub style: Style,
    /// The order of the profile menu, and which profiles it hides.
    pub menu: Arrangement,
    /// The user's profiles, and their changes to presets, in file order.
    pub profiles: Vec<ProfileSpec>,
}

impl Settings {
    /// Parses and validates `settings.toml`.
    pub fn from_toml(source: &str) -> Result<Self, SettingsError> {
        let file: schema::SettingsFile = toml::from_str(source)?;
        file.validate()
    }

    /// Saves `spec` as one of the user's profiles, replacing the one with the
    /// same name. A preset of that name is replaced in turn.
    pub fn put_profile(&mut self, spec: ProfileSpec) {
        match self.profiles.iter_mut().find(|p| p.name == spec.name) {
            Some(existing) => *existing = spec,
            None => self.profiles.push(spec),
        }
    }

    /// Removes one of the user's profiles. For a changed preset, this restores
    /// the preset. Returns whether there was one.
    pub fn remove_profile(&mut self, name: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        self.profiles.len() != before
    }
}

/// Light or dark.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ThemeMode {
    /// Follow the desktop or phone setting.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// The name used in `settings.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }

    /// A label for settings screens.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The allowed range and default of a numeric setting. Settings screens use it
/// for their sliders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Limits {
    pub min: u32,
    pub max: u32,
    pub default: u32,
}

impl Limits {
    pub fn contains(self, value: i64) -> bool {
        (i64::from(self.min)..=i64::from(self.max)).contains(&value)
    }

    pub fn clamp(self, value: u32) -> u32 {
        value.clamp(self.min, self.max)
    }
}

/// How keys behave and how big they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeySettings {
    /// Milliseconds before a long-press fires a key's hold action.
    pub hold_delay_ms: u32,
    /// Milliseconds before a held repeating key starts repeating.
    pub repeat_delay_ms: u32,
    /// Repeats per second.
    pub repeat_rate: u32,
    /// Pixels of drag per swipe step.
    pub swipe_step_px: u32,
    /// Key height in pixels; `None` sizes keys to the screen.
    pub height_px: Option<u32>,
}

impl KeySettings {
    pub const HOLD_DELAY_MS: Limits = Limits {
        min: 150,
        max: 1000,
        default: 300,
    };
    pub const REPEAT_DELAY_MS: Limits = Limits {
        min: 150,
        max: 1000,
        default: 400,
    };
    pub const REPEAT_RATE: Limits = Limits {
        min: 5,
        max: 60,
        default: 30,
    };
    pub const SWIPE_STEP_PX: Limits = Limits {
        min: 4,
        max: 40,
        default: 12,
    };
    pub const HEIGHT_PX: Limits = Limits {
        min: 32,
        max: 96,
        default: 48,
    };

    /// The gesture thresholds these settings describe.
    pub fn gesture_config(&self) -> GestureConfig {
        let defaults = GestureConfig::default();
        let swipe_step = self.swipe_step_px as f32;
        GestureConfig {
            hold_delay: Duration::from_millis(self.hold_delay_ms.into()),
            repeat_delay: Duration::from_millis(self.repeat_delay_ms.into()),
            repeat_interval: Duration::from_millis(u64::from(1000 / self.repeat_rate.max(1))),
            swipe_step,
            // A swipe should not need more travel to start than to take a step.
            swipe_slop: defaults.swipe_slop.min(swipe_step),
        }
    }
}

impl Default for KeySettings {
    fn default() -> Self {
        Self {
            hold_delay_ms: Self::HOLD_DELAY_MS.default,
            repeat_delay_ms: Self::REPEAT_DELAY_MS.default,
            repeat_rate: Self::REPEAT_RATE.default,
            swipe_step_px: Self::SWIPE_STEP_PX.default,
            height_px: None,
        }
    }
}

/// The in-app preview panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreviewSettings {
    /// Show the log of recent output beside the preview text.
    pub show_log: bool,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self { show_log: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{StyleValue, StyleVar, ValueKind};

    #[test]
    fn empty_file_is_the_defaults() {
        assert_eq!(Settings::from_toml("").unwrap(), Settings::default());
    }

    #[test]
    fn default_key_settings_match_the_engine_defaults() {
        assert_eq!(
            KeySettings::default().gesture_config(),
            GestureConfig::default()
        );
    }

    #[test]
    fn parses_every_field() {
        let settings = Settings::from_toml(
            r##"
            profile = "mine"
            theme = "light"
            [keys]
            hold-delay = 500
            repeat-delay = 250
            repeat-rate = 20
            swipe-step = 8
            height = 56
            [preview]
            show-log = false
            [style]
            radius = "10px"
            [style.dark]
            accent = "#ff9e64"
            [style.light]
            accent = "#c2410c"
            "##,
        )
        .unwrap();
        assert_eq!(settings.profile.as_deref(), Some("mine"));
        assert_eq!(settings.theme, ThemeMode::Light);
        assert_eq!(
            settings.keys,
            KeySettings {
                hold_delay_ms: 500,
                repeat_delay_ms: 250,
                repeat_rate: 20,
                swipe_step_px: 8,
                height_px: Some(56),
            }
        );
        assert!(!settings.preview.show_log);
        let value = |kind, v: &str| StyleValue::parse(kind, v).unwrap();
        assert_eq!(
            settings.style.both.get(&StyleVar::Radius),
            Some(&value(ValueKind::Length, "10px"))
        );
        assert_eq!(
            settings.style.dark.get(&StyleVar::Accent),
            Some(&value(ValueKind::Color, "#ff9e64"))
        );
        assert_eq!(settings.style.light.len(), 1);

        let config = settings.keys.gesture_config();
        assert_eq!(config.repeat_interval, Duration::from_millis(50));
        assert_eq!(config.swipe_slop, 8.0);
    }

    #[test]
    fn limits() {
        let limits = KeySettings::REPEAT_RATE;
        assert!(limits.contains(5) && limits.contains(60));
        assert!(!limits.contains(4) && !limits.contains(61) && !limits.contains(-1));
        assert_eq!(limits.clamp(100), 60);
    }

    #[test]
    fn theme_names_round_trip() {
        for mode in ThemeMode::ALL {
            assert_eq!(ThemeMode::from_name(mode.name()), Some(mode));
        }
    }
}
