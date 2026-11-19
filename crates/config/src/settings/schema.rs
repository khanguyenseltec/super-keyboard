//! The shape of `settings.toml`, and its validation into [`Settings`].

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{
    Align, KeySettings, KeyboardSettings, Limits, PreviewSettings, Settings, SettingsError,
    ThemeChoice, ThemeSettings,
};
use crate::{
    profile::{ProfileFile, is_valid_name, validate_profiles},
    style::{Style, StyleValue, StyleVar},
};
use super_keyboard_core::{Arrangement, InputMethod, ToneStyle, VietnameseOptions};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct SettingsFile {
    profile: Option<String>,
    theme: Option<String>,
    light_theme: Option<String>,
    dark_theme: Option<String>,
    #[serde(default)]
    keys: KeysFile,
    #[serde(default)]
    keyboard: KeyboardFile,
    #[serde(default)]
    preview: PreviewFile,
    #[serde(default)]
    vietnamese: VietnameseFile,
    #[serde(default)]
    style: StyleFile,
    #[serde(default)]
    menu: MenuFile,
    #[serde(default)]
    profiles: Vec<ProfileFile>,
}

/// `[menu]`: the order of the profile menu, and which entries it hides.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct MenuFile {
    #[serde(default)]
    order: Vec<String>,
    #[serde(default)]
    shown: BTreeMap<String, bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct KeysFile {
    hold_delay: Option<i64>,
    repeat_delay: Option<i64>,
    repeat_rate: Option<i64>,
    swipe_step: Option<i64>,
    /// `"auto"` or a number of pixels.
    height: Option<toml::Value>,
    auto_pair: Option<bool>,
}

/// `[keyboard]`: its share of the window, and where it sits.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyboardFile {
    /// `"auto"` or a percentage.
    width: Option<toml::Value>,
    /// `"auto"` or a percentage.
    height: Option<toml::Value>,
    align: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct PreviewFile {
    show_log: Option<bool>,
}

/// `[vietnamese]`: Telex or VNI typing.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct VietnameseFile {
    enabled: Option<bool>,
    method: Option<String>,
    tone_style: Option<String>,
    check_spelling: Option<bool>,
}

impl VietnameseFile {
    fn validate(self) -> Result<VietnameseOptions, SettingsError> {
        let defaults = VietnameseOptions::default();
        Ok(VietnameseOptions {
            enabled: self.enabled.unwrap_or(defaults.enabled),
            method: match self.method {
                None => defaults.method,
                Some(name) => {
                    InputMethod::from_name(&name).ok_or(SettingsError::UnknownInputMethod(name))?
                }
            },
            tone_style: match self.tone_style {
                None => defaults.tone_style,
                Some(name) => {
                    ToneStyle::from_name(&name).ok_or(SettingsError::UnknownToneStyle(name))?
                }
            },
            check_spelling: self.check_spelling.unwrap_or(defaults.check_spelling),
        })
    }
}

/// `[style]` holds variables for every theme; `[style.dark]` and
/// `[style.light]` hold variables for dark or light themes only. (Unknown names land in `both` and are
/// reported by validation, so `deny_unknown_fields` is not needed here.)
#[derive(Debug, Default, Deserialize)]
struct StyleFile {
    #[serde(default)]
    dark: BTreeMap<String, String>,
    #[serde(default)]
    light: BTreeMap<String, String>,
    #[serde(flatten)]
    both: BTreeMap<String, String>,
}

impl SettingsFile {
    pub fn validate(self) -> Result<Settings, SettingsError> {
        let profile = match self.profile {
            Some(profile) if profile.trim().is_empty() => return Err(SettingsError::EmptyProfile),
            profile => profile,
        };
        let defaults = ThemeSettings::default();
        let theme = ThemeSettings {
            choice: match self.theme {
                None => ThemeChoice::System,
                Some(name) => ThemeChoice::from_name(&theme_name("theme", name)?),
            },
            light: match self.light_theme {
                None => defaults.light,
                Some(name) => theme_name("light-theme", name)?,
            },
            dark: match self.dark_theme {
                None => defaults.dark,
                Some(name) => theme_name("dark-theme", name)?,
            },
        };
        Ok(Settings {
            profile,
            theme,
            keys: self.keys.validate()?,
            keyboard: self.keyboard.validate()?,
            preview: PreviewSettings {
                show_log: self
                    .preview
                    .show_log
                    .unwrap_or(PreviewSettings::default().show_log),
            },
            vietnamese: self.vietnamese.validate()?,
            style: self.style.validate()?,
            menu: Arrangement {
                order: self.menu.order,
                enabled: self.menu.shown,
            },
            profiles: validate_profiles(self.profiles)?,
        })
    }
}

impl KeysFile {
    fn validate(self) -> Result<KeySettings, SettingsError> {
        Ok(KeySettings {
            hold_delay_ms: number(
                "keys.hold-delay",
                self.hold_delay,
                KeySettings::HOLD_DELAY_MS,
            )?,
            repeat_delay_ms: number(
                "keys.repeat-delay",
                self.repeat_delay,
                KeySettings::REPEAT_DELAY_MS,
            )?,
            repeat_rate: number(
                "keys.repeat-rate",
                self.repeat_rate,
                KeySettings::REPEAT_RATE,
            )?,
            swipe_step_px: number(
                "keys.swipe-step",
                self.swipe_step,
                KeySettings::SWIPE_STEP_PX,
            )?,
            height_px: auto_or_number("keys.height", self.height, KeySettings::HEIGHT_PX)?,
            auto_pair: self.auto_pair.unwrap_or(KeySettings::default().auto_pair),
        })
    }
}

impl KeyboardFile {
    fn validate(self) -> Result<KeyboardSettings, SettingsError> {
        Ok(KeyboardSettings {
            width_pct: auto_or_number("keyboard.width", self.width, KeyboardSettings::WIDTH_PCT)?,
            height_pct: auto_or_number(
                "keyboard.height",
                self.height,
                KeyboardSettings::HEIGHT_PCT,
            )?,
            align: match self.align {
                None => Align::default(),
                Some(name) => Align::from_name(&name).ok_or(SettingsError::UnknownAlign(name))?,
            },
        })
    }
}

/// A theme name: `"system"`, or letters, digits, `-` and `_` like a theme's
/// file name.
fn theme_name(field: &'static str, name: String) -> Result<String, SettingsError> {
    let system_allowed = field == "theme";
    if is_valid_name(&name) && (system_allowed || name != ThemeChoice::SYSTEM) {
        Ok(name)
    } else {
        Err(SettingsError::InvalidThemeName { field, name })
    }
}

/// `"auto"` (`None`) or a number within `limits`.
fn auto_or_number(
    field: &'static str,
    value: Option<toml::Value>,
    limits: Limits,
) -> Result<Option<u32>, SettingsError> {
    match value {
        None => Ok(None),
        Some(toml::Value::String(word)) if word == "auto" => Ok(None),
        Some(toml::Value::Integer(n)) => number(field, Some(n), limits).map(Some),
        Some(_) => Err(SettingsError::NotAutoOrNumber { field }),
    }
}

fn number(field: &'static str, value: Option<i64>, limits: Limits) -> Result<u32, SettingsError> {
    match value {
        None => Ok(limits.default),
        Some(value) if limits.contains(value) => {
            Ok(u32::try_from(value).expect("limits are within u32"))
        }
        Some(value) => Err(SettingsError::OutOfRange {
            field,
            value,
            limits,
        }),
    }
}

impl StyleFile {
    fn validate(self) -> Result<Style, SettingsError> {
        Ok(Style {
            both: style_table("style", self.both)?,
            dark: style_table("style.dark", self.dark)?,
            light: style_table("style.light", self.light)?,
        })
    }
}

fn style_table(
    section: &str,
    table: BTreeMap<String, String>,
) -> Result<BTreeMap<StyleVar, StyleValue>, SettingsError> {
    table
        .into_iter()
        .map(|(name, value)| {
            let path = format!("{section}.{name}");
            let Some(var) = StyleVar::from_name(&name) else {
                let known = StyleVar::ALL.map(StyleVar::name).join(", ");
                return Err(SettingsError::UnknownStyleVar { path, known });
            };
            let value = StyleValue::parse(var.kind(), &value)
                .map_err(|reason| SettingsError::InvalidStyleValue { path, reason })?;
            Ok((var, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{KeySettings, Settings, SettingsError};

    fn error(source: &str) -> String {
        Settings::from_toml(source).unwrap_err().to_string()
    }

    #[test]
    fn reports_each_mistake() {
        assert!(error("thme = \"dark\"").contains("unknown field `thme`"));
        assert!(error("[keys]\nhold = 3").contains("unknown field `hold`"));
        assert!(error("[keys]\nhold-delay = \"slow\"").contains("invalid type"));
        assert_eq!(
            error("theme = \"Solarized Dark\""),
            r#"`theme` must be "system" or a theme's name (letters, digits, `-` and `_`), not "Solarized Dark""#
        );
        assert_eq!(
            error("dark-theme = \"system\""),
            r#"`dark-theme` must be a theme's name (letters, digits, `-` and `_`), not "system""#
        );
        assert_eq!(error("profile = \" \""), "`profile` must not be empty");
        assert_eq!(
            error("[[profiles]]\nname = \"a b\"\nlayout = \"dev\""),
            "profile name `a b` may only use letters, digits, `-` and `_`"
        );
        assert_eq!(
            error(
                "[[profiles]]\nname = \"a\"\nlayout = \"dev\"\n[[profiles]]\nname = \"a\"\nlayout = \"dev\""
            ),
            "profile `a` is defined more than once"
        );
        assert_eq!(
            error("[[profiles]]\nname = \"a\"\nlayout = \"\""),
            "profile `a` needs a `layout`"
        );
        assert!(error("[[profiles]]\nname = \"a\"").contains("missing field `layout`"));
        assert!(error("[menu]\nhidden = []").contains("unknown field `hidden`"));
        assert_eq!(
            error("[keys]\nrepeat-rate = 0"),
            "`keys.repeat-rate` must be between 5 and 60, not 0"
        );
        assert_eq!(
            error("[keys]\nheight = 500"),
            "`keys.height` must be between 32 and 96, not 500"
        );
        assert_eq!(
            error("[keys]\nheight = \"big\""),
            r#"`keys.height` must be "auto" or a number of pixels"#
        );
        assert_eq!(
            Settings::from_toml("[keys]\nheight = \"auto\"").unwrap(),
            Settings::default()
        );
        assert_eq!(
            error("[keyboard]\nwidth = 30"),
            "`keyboard.width` must be between 40 and 100, not 30"
        );
        assert_eq!(
            error("[keyboard]\nheight = \"50%\""),
            r#"`keyboard.height` must be "auto" or a percentage of the window"#
        );
        assert_eq!(
            error("[keyboard]\nalign = \"top\""),
            r#"`keyboard.align` must be "center", "left" or "right", not "top""#
        );
        assert!(error("[keyboard]\nsize = 3").contains("unknown field `size`"));
        assert_eq!(
            error("[vietnamese]\nmethod = \"viqr\""),
            r#"`vietnamese.method` must be "telex" or "vni", not "viqr""#
        );
        assert_eq!(
            error("[vietnamese]\ntone-style = \"new\""),
            r#"`vietnamese.tone-style` must be "classic" or "modern", not "new""#
        );
        assert!(error("[vietnamese]\nenable = true").contains("unknown field `enable`"));
        assert_eq!(
            Settings::from_toml("[keyboard]\nwidth = \"auto\"\nheight = \"auto\"").unwrap(),
            Settings::default()
        );
        assert!(
            error("[style]\nacent = \"red\"").starts_with("unknown style variable `style.acent`")
        );
        assert_eq!(
            error("[style.dark]\nbg = \"url(x)\""),
            "`style.dark.bg`: function `url()` is not allowed"
        );
        assert!(error("[style.dim]\nbg = \"red\"").contains("invalid type"));
        assert_eq!(
            error("[style]\nfont-family = \"'Fira Code'\""),
            "`style.font-family`: write font names without quotes, separated by commas"
        );
    }

    #[test]
    fn range_errors_use_the_documented_limits() {
        let err = Settings::from_toml("[keys]\nswipe-step = 100").unwrap_err();
        assert_eq!(
            err,
            SettingsError::OutOfRange {
                field: "keys.swipe-step",
                value: 100,
                limits: KeySettings::SWIPE_STEP_PX,
            }
        );
    }
}
