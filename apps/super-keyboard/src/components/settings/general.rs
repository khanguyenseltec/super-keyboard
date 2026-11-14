//! General settings: theme, key timing and size, and the preview log.

use dioxus::prelude::*;
use super_keyboard_config::{KeySettings, Settings, ThemeMode};

use super::fields::{RangeField, SelectField, ToggleField};
use crate::state::AppState;

#[component]
pub fn GeneralSettings() -> Element {
    let mut state = use_context::<AppState>();
    let settings = state.config.read().settings.clone();
    let keys = settings.keys;
    let themes: Vec<(String, String)> = ThemeMode::ALL
        .iter()
        .map(|mode| (mode.name().to_owned(), mode.label().to_owned()))
        .collect();

    rsx! {
        div { class: "settings-tab",
            div { class: "fields",
                SelectField {
                    label: "Theme",
                    value: settings.theme.name(),
                    options: themes,
                    onchange: move |name: String| {
                        let theme = ThemeMode::from_name(&name).unwrap_or_default();
                        state.update_settings(|s| s.theme = theme);
                    },
                }
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
                ToggleField {
                    label: "Fit keys to screen",
                    checked: keys.height_px.is_none(),
                    onchange: move |fit: bool| {
                        let height = (!fit).then_some(KeySettings::HEIGHT_PX.default);
                        state.update_settings(|s| s.keys.height_px = height);
                    },
                }
                RangeField {
                    label: "Key height",
                    unit: "px",
                    limits: KeySettings::HEIGHT_PX,
                    step: 2,
                    value: keys.height_px.unwrap_or(KeySettings::HEIGHT_PX.default),
                    disabled: keys.height_px.is_none(),
                    onchange: move |px| state.update_settings(|s| s.keys.height_px = Some(px)),
                }
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

/// Resets what this tab controls. Profiles, the menu and hand-written styles
/// are kept.
fn reset_panel_settings(settings: &mut Settings) {
    *settings = Settings {
        profile: settings.profile.take(),
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
    fn reset_keeps_profiles_and_styles() {
        let mut settings = Settings::from_toml(
            "profile = \"mine\"\ntheme = \"dark\"\n[keys]\nrepeat-rate = 10\n[style]\ngap = \"2px\"\n\
             [menu]\norder = [\"mine\"]\n[[profiles]]\nname = \"mine\"\nlayout = \"dev\"",
        )
        .unwrap();
        let kept = settings.clone();
        reset_panel_settings(&mut settings);
        assert_eq!(
            settings,
            Settings {
                profile: kept.profile,
                style: kept.style,
                menu: kept.menu,
                profiles: kept.profiles,
                ..Settings::default()
            }
        );
    }
}
