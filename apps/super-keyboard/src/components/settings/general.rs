//! General settings: key timing, keyboard size and position, and the preview
//! log.

use dioxus::prelude::*;
use super_keyboard_config::{Align, KeySettings, KeyboardSettings, Settings};

use super::fields::{RangeField, SelectField, ToggleField};
use crate::state::AppState;

/// How the keyboard's height is decided: one setting on screen, two in
/// `settings.toml` (`keys.height` and `keyboard.height`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeightMode {
    /// Keys follow the screen height.
    Auto,
    /// Every key is `keys.height` pixels tall.
    KeyHeight,
    /// The keyboard fills `keyboard.height` percent of the window.
    WindowShare,
}

impl HeightMode {
    const ALL: [Self; 3] = [Self::Auto, Self::KeyHeight, Self::WindowShare];

    fn of(settings: &Settings) -> Self {
        match (settings.keyboard.height_pct, settings.keys.height_px) {
            (Some(_), _) => Self::WindowShare,
            (None, Some(_)) => Self::KeyHeight,
            (None, None) => Self::Auto,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::KeyHeight => "key",
            Self::WindowShare => "share",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Auto => "Fit to screen",
            Self::KeyHeight => "Fixed key height",
            Self::WindowShare => "Share of window",
        }
    }

    fn from_name(name: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name() == name)
            .unwrap_or(Self::Auto)
    }

    /// Switches `settings` to this mode, starting from the default size.
    fn apply(self, settings: &mut Settings) {
        let (keys, keyboard) = (&mut settings.keys, &mut settings.keyboard);
        match self {
            Self::Auto => {
                keys.height_px = None;
                keyboard.height_pct = None;
            }
            Self::KeyHeight => {
                keys.height_px = Some(keys.height_px.unwrap_or(KeySettings::HEIGHT_PX.default));
                keyboard.height_pct = None;
            }
            Self::WindowShare => {
                keyboard.height_pct = Some(
                    keyboard
                        .height_pct
                        .unwrap_or(KeyboardSettings::HEIGHT_PCT.default),
                );
            }
        }
    }
}

#[component]
pub fn GeneralSettings() -> Element {
    let mut state = use_context::<AppState>();
    let settings = state.config.read().settings.clone();
    let (keys, keyboard) = (settings.keys, settings.keyboard);
    let height_mode = HeightMode::of(&settings);
    let height_modes: Vec<(String, String)> = HeightMode::ALL
        .iter()
        .map(|mode| (mode.name().to_owned(), mode.label().to_owned()))
        .collect();
    let aligns: Vec<(String, String)> = Align::ALL
        .iter()
        .map(|align| (align.name().to_owned(), align.label().to_owned()))
        .collect();

    rsx! {
        div { class: "settings-tab",
            h3 { class: "group-title", "Keys" }
            div { class: "fields",
                RangeField {
                    label: "Long-press delay",
                    unit: "ms",
                    limits: KeySettings::HOLD_DELAY_MS,
                    step: 10,
                    value: keys.hold_delay_ms,
                    onchange: move |ms| state.update_settings(|s| s.keys.hold_delay_ms = ms),
                }
                RangeField {
                    label: "Repeat delay",
                    unit: "ms",
                    limits: KeySettings::REPEAT_DELAY_MS,
                    step: 10,
                    value: keys.repeat_delay_ms,
                    onchange: move |ms| state.update_settings(|s| s.keys.repeat_delay_ms = ms),
                }
                RangeField {
                    label: "Repeat rate",
                    unit: "/s",
                    limits: KeySettings::REPEAT_RATE,
                    step: 1,
                    value: keys.repeat_rate,
                    onchange: move |rate| state.update_settings(|s| s.keys.repeat_rate = rate),
                }
                RangeField {
                    label: "Swipe step",
                    unit: "px",
                    limits: KeySettings::SWIPE_STEP_PX,
                    step: 1,
                    value: keys.swipe_step_px,
                    onchange: move |px| state.update_settings(|s| s.keys.swipe_step_px = px),
                }
            }
            h3 { class: "group-title", "Keyboard size" }
            div { class: "fields",
                SelectField {
                    label: "Height",
                    value: height_mode.name(),
                    options: height_modes,
                    onchange: move |name: String| {
                        state.update_settings(|s| HeightMode::from_name(&name).apply(s));
                    },
                }
                RangeField {
                    label: "Key height",
                    unit: "px",
                    limits: KeySettings::HEIGHT_PX,
                    step: 2,
                    value: keys.height_px.unwrap_or(KeySettings::HEIGHT_PX.default),
                    disabled: height_mode != HeightMode::KeyHeight,
                    onchange: move |px| state.update_settings(|s| s.keys.height_px = Some(px)),
                }
                RangeField {
                    label: "Window share",
                    unit: "%",
                    limits: KeyboardSettings::HEIGHT_PCT,
                    step: 5,
                    value: keyboard.height_pct.unwrap_or(KeyboardSettings::HEIGHT_PCT.default),
                    disabled: height_mode != HeightMode::WindowShare,
                    onchange: move |pct| state.update_settings(|s| s.keyboard.height_pct = Some(pct)),
                }
                ToggleField {
                    label: "Full width",
                    checked: keyboard.width_pct.is_none(),
                    onchange: move |full: bool| {
                        let width = (!full).then_some(KeyboardSettings::WIDTH_PCT.default);
                        state.update_settings(|s| s.keyboard.width_pct = width);
                    },
                }
                RangeField {
                    label: "Width",
                    unit: "%",
                    limits: KeyboardSettings::WIDTH_PCT,
                    step: 5,
                    value: keyboard.width_pct.unwrap_or(KeyboardSettings::WIDTH_PCT.default),
                    disabled: keyboard.width_pct.is_none(),
                    onchange: move |pct| state.update_settings(|s| s.keyboard.width_pct = Some(pct)),
                }
                SelectField {
                    label: "Position",
                    value: keyboard.align.name(),
                    options: aligns,
                    onchange: move |name: String| {
                        let align = Align::from_name(&name).unwrap_or_default();
                        state.update_settings(|s| s.keyboard.align = align);
                    },
                }
            }
            p { class: "hint",
                "A share of the window stretches the keys to fill it; at 100% the keyboard fills the window and grows with it."
            }
            h3 { class: "group-title", "Preview" }
            div { class: "fields",
                ToggleField {
                    label: "Show output log",
                    checked: settings.preview.show_log,
                    onchange: move |show: bool| state.update_settings(|s| s.preview.show_log = show),
                }
            }
            div { class: "actions",
                button {
                    class: "bar-button",
                    onclick: move |_| state.update_settings(reset_panel_settings),
                    "reset to defaults"
                }
            }
        }
    }
}

/// Resets what this tab controls. The theme, profiles, the menu and
/// hand-written styles are kept.
fn reset_panel_settings(settings: &mut Settings) {
    *settings = Settings {
        profile: settings.profile.take(),
        theme: std::mem::take(&mut settings.theme),
        style: std::mem::take(&mut settings.style),
        menu: std::mem::take(&mut settings.menu),
        profiles: std::mem::take(&mut settings.profiles),
        ..Settings::default()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_keeps_the_theme_profiles_and_styles() {
        let mut settings = Settings::from_toml(
            "profile = \"mine\"\ntheme = \"nord\"\n[keys]\nrepeat-rate = 10\n[keyboard]\nwidth = 50\n\
             [style]\ngap = \"2px\"\n[menu]\norder = [\"mine\"]\n\
             [[profiles]]\nname = \"mine\"\nlayout = \"dev\"",
        )
        .unwrap();
        let kept = settings.clone();
        reset_panel_settings(&mut settings);
        assert_eq!(
            settings,
            Settings {
                profile: kept.profile,
                theme: kept.theme,
                style: kept.style,
                menu: kept.menu,
                profiles: kept.profiles,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn one_height_setting_drives_two_fields() {
        let mut settings = Settings::default();
        assert_eq!(HeightMode::of(&settings), HeightMode::Auto);

        HeightMode::KeyHeight.apply(&mut settings);
        assert_eq!(
            settings.keys.height_px,
            Some(KeySettings::HEIGHT_PX.default)
        );
        assert_eq!(HeightMode::of(&settings), HeightMode::KeyHeight);

        settings.keys.height_px = Some(60);
        HeightMode::WindowShare.apply(&mut settings);
        assert_eq!(
            settings.keyboard.height_pct,
            Some(KeyboardSettings::HEIGHT_PCT.default)
        );
        assert_eq!(HeightMode::of(&settings), HeightMode::WindowShare);

        // Back to a fixed key height: the earlier height is still there.
        HeightMode::KeyHeight.apply(&mut settings);
        assert_eq!(settings.keys.height_px, Some(60));
        assert_eq!(settings.keyboard.height_pct, None);

        HeightMode::Auto.apply(&mut settings);
        assert_eq!(settings, Settings::default());
        for mode in HeightMode::ALL {
            assert_eq!(HeightMode::from_name(mode.name()), mode);
        }
    }
}
