//! Themes: colours, fonts and sizes, written as JSON.
//!
//! ```json
//! {
//!   "title": "Midnight",
//!   "appearance": "dark",
//!   "colors": { "bg": "#0b1020", "key": "#1b2340", "accent": "#ffb454" },
//!   "fonts": {
//!     "ui":   { "family": ["Inter", "sans-serif"], "size": "13px" },
//!     "keys": { "family": ["Fira Code", "monospace"], "weight": 500 }
//!   },
//!   "sizes": { "radius": "10px" }
//! }
//! ```
//!
//! A theme's name is its file name without `.json`. Everything but `title` and
//! `appearance` is optional: what a theme leaves out comes from the built-in
//! theme of the same appearance (`dark` or `light`), except key labels, which
//! follow the theme's own `text` and `muted`. Values are checked like `[style]`
//! values ([`StyleValue`]), so a theme can never load anything: fonts are looked
//! up among the installed ones and never downloaded.
//!
//! [`BUILTIN`] lists the themes that ship with the app; `themes/*.json` in the
//! config directory adds more, and a file named like a built-in theme replaces
//! it. See `docs/CUSTOMIZING.md` and `themes/theme.schema.json`.

mod library;

pub use library::{ThemeEntry, ThemeLibrary, ThemeOrigin};

use std::{collections::BTreeMap, fmt, sync::LazyLock};

use serde::{Deserialize, de::IgnoredAny};

use crate::style::{FontRole, StyleValue, StyleVar, ValueKind};

/// Light or dark: which `[style.light]` or `[style.dark]` overrides apply, and
/// which slot a theme fills when the theme follows the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Self; 2] = [Self::Light, Self::Dark];

    /// The name used in theme files, and in CSS (`color-scheme`,
    /// `prefers-color-scheme`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }

    /// A label for settings screens.
    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    /// The built-in theme of this appearance, which fills in what other
    /// themes leave out.
    pub fn default_theme(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl fmt::Display for Appearance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A theme, ready to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// What the theme list shows.
    pub title: String,
    pub description: String,
    pub appearance: Appearance,
    /// Every style variable, with what the file leaves out filled in.
    pub vars: BTreeMap<StyleVar, StyleValue>,
}

/// Why a theme file cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThemeError {
    /// Not valid JSON, an unknown field or a value of the wrong type. The
    /// message includes the line and column.
    #[error("{0}")]
    Json(String),
    #[error("`title` must not be empty")]
    EmptyTitle,
    #[error("`appearance` must be \"light\" or \"dark\", not {0:?}")]
    UnknownAppearance(String),
    #[error("`{path}` is not a theme {what} (known: {known})")]
    UnknownVar {
        path: String,
        what: &'static str,
        known: String,
    },
    #[error("`{path}`: {reason}")]
    InvalidValue { path: String, reason: String },
}

impl Theme {
    /// Parses and checks a theme file, and fills in what it leaves out.
    pub fn from_json(source: &str) -> Result<Self, ThemeError> {
        let mut theme = Self::parse(source)?;
        let set = theme.vars.clone();
        let base = &base(theme.appearance).vars;
        for var in StyleVar::ALL {
            if set.contains_key(&var) {
                continue;
            }
            let value = var
                .fallback()
                .and_then(|fallback| set.get(&fallback))
                .or_else(|| base.get(&var));
            if let Some(value) = value {
                theme.vars.insert(var, value.clone());
            }
        }
        Ok(theme)
    }

    /// Parses and checks a theme file, keeping only what it sets.
    fn parse(source: &str) -> Result<Self, ThemeError> {
        let file: ThemeFile =
            serde_json::from_str(source).map_err(|err| ThemeError::Json(err.to_string()))?;
        file.validate()
    }

    /// The value of `var`. Every theme from [`Theme::from_json`] has them all.
    pub fn get(&self, var: StyleVar) -> Option<&StyleValue> {
        self.vars.get(&var)
    }
}

/// The themes that ship with the app: their names and their files.
pub const BUILTIN: [(&str, &str); 24] = [
    ("dark", include_str!("../../themes/dark.json")),
    ("light", include_str!("../../themes/light.json")),
    ("tokyo-night", include_str!("../../themes/tokyo-night.json")),
    ("dracula", include_str!("../../themes/dracula.json")),
    ("nord", include_str!("../../themes/nord.json")),
    (
        "gruvbox-dark",
        include_str!("../../themes/gruvbox-dark.json"),
    ),
    (
        "gruvbox-light",
        include_str!("../../themes/gruvbox-light.json"),
    ),
    (
        "solarized-dark",
        include_str!("../../themes/solarized-dark.json"),
    ),
    (
        "solarized-light",
        include_str!("../../themes/solarized-light.json"),
    ),
    (
        "catppuccin-mocha",
        include_str!("../../themes/catppuccin-mocha.json"),
    ),
    (
        "catppuccin-latte",
        include_str!("../../themes/catppuccin-latte.json"),
    ),
    ("one-dark", include_str!("../../themes/one-dark.json")),
    ("rose-pine", include_str!("../../themes/rose-pine.json")),
    (
        "rose-pine-dawn",
        include_str!("../../themes/rose-pine-dawn.json"),
    ),
    (
        "everforest-dark",
        include_str!("../../themes/everforest-dark.json"),
    ),
    ("kanagawa", include_str!("../../themes/kanagawa.json")),
    ("aubergine", include_str!("../../themes/aubergine.json")),
    ("oled", include_str!("../../themes/oled.json")),
    ("amber", include_str!("../../themes/amber.json")),
    ("phosphor", include_str!("../../themes/phosphor.json")),
    ("neon", include_str!("../../themes/neon.json")),
    ("paper", include_str!("../../themes/paper.json")),
    (
        "high-contrast-dark",
        include_str!("../../themes/high-contrast-dark.json"),
    ),
    (
        "high-contrast-light",
        include_str!("../../themes/high-contrast-light.json"),
    ),
];

/// The file of the built-in theme `id`, to start a theme of one's own from.
pub fn builtin_source(id: &str) -> Option<&'static str> {
    BUILTIN
        .iter()
        .find(|(name, _)| *name == id)
        .map(|(_, source)| *source)
}

/// The built-in themes, parsed once.
pub(crate) fn builtin_themes() -> &'static [(&'static str, Theme)] {
    static THEMES: LazyLock<Vec<(&str, Theme)>> = LazyLock::new(|| {
        BUILTIN
            .iter()
            .map(|(id, source)| {
                let theme = Theme::from_json(source)
                    .unwrap_or_else(|err| panic!("built-in theme `{id}` is invalid: {err}"));
                (*id, theme)
            })
            .collect()
    });
    &THEMES
}

/// The built-in `dark` or `light` theme as written. Both set every variable,
/// so they are what other themes are filled in from.
fn base(appearance: Appearance) -> &'static Theme {
    static BASES: LazyLock<[Theme; 2]> = LazyLock::new(|| {
        Appearance::ALL.map(|appearance| {
            let source = builtin_source(appearance.default_theme()).expect("listed in BUILTIN");
            Theme::parse(source).expect("built-in base theme is valid")
        })
    });
    &BASES[Appearance::ALL
        .iter()
        .position(|a| *a == appearance)
        .expect("in ALL")]
}

/// A theme file as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    /// For editors that complete and check theme files; ignored.
    #[serde(rename = "$schema", default)]
    _schema: Option<IgnoredAny>,
    title: String,
    #[serde(default)]
    description: String,
    appearance: String,
    #[serde(default)]
    colors: BTreeMap<String, String>,
    #[serde(default)]
    fonts: FontsFile,
    #[serde(default)]
    sizes: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FontsFile {
    ui: Option<FontFile>,
    keys: Option<FontFile>,
    preview: Option<FontFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FontFile {
    family: Option<Family>,
    size: Option<String>,
    weight: Option<Weight>,
}

/// `["Fira Code", "monospace"]` or `"Fira Code, monospace"`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Family {
    List(Vec<String>),
    Text(String),
}

/// `500` or `"bold"`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Weight {
    Number(i64),
    Text(String),
}

/// The sizes a theme can set; font sizes go under `fonts`.
const SIZES: [StyleVar; 2] = [StyleVar::Radius, StyleVar::Gap];

impl ThemeFile {
    fn validate(self) -> Result<Theme, ThemeError> {
        let title = self.title.trim().to_owned();
        if title.is_empty() {
            return Err(ThemeError::EmptyTitle);
        }
        let appearance = Appearance::from_name(&self.appearance)
            .ok_or(ThemeError::UnknownAppearance(self.appearance))?;
        let mut vars = BTreeMap::new();

        let colors: Vec<StyleVar> = StyleVar::ALL
            .into_iter()
            .filter(|var| var.kind() == ValueKind::Color)
            .collect();
        for (name, value) in self.colors {
            let path = format!("colors.{name}");
            let var = StyleVar::from_name(&name)
                .filter(|var| colors.contains(var))
                .ok_or_else(|| unknown(path.clone(), "colour", &colors))?;
            vars.insert(var, value_of(var, &path, &value)?);
        }
        for (name, value) in self.sizes {
            let path = format!("sizes.{name}");
            let var = StyleVar::from_name(&name)
                .filter(|var| SIZES.contains(var))
                .ok_or_else(|| unknown(path.clone(), "size", &SIZES))?;
            vars.insert(var, value_of(var, &path, &value)?);
        }
        let fonts = [self.fonts.ui, self.fonts.keys, self.fonts.preview];
        for (role, font) in FontRole::ALL.into_iter().zip(fonts) {
            let Some(font) = font else { continue };
            let [family_var, size_var, weight_var] = role.vars();
            let path = |field: &str| format!("fonts.{}.{field}", role.name());
            if let Some(family) = font.family {
                let family = match &family {
                    Family::List(names) => {
                        StyleValue::font_family(names.iter().map(String::as_str))
                    }
                    Family::Text(text) => StyleValue::parse(ValueKind::FontFamily, text),
                }
                .map_err(|reason| invalid(path("family"), reason))?;
                vars.insert(family_var, family);
            }
            if let Some(size) = font.size {
                vars.insert(size_var, value_of(size_var, &path("size"), &size)?);
            }
            if let Some(weight) = font.weight {
                let text = match weight {
                    Weight::Number(n) => n.to_string(),
                    Weight::Text(text) => text,
                };
                vars.insert(weight_var, value_of(weight_var, &path("weight"), &text)?);
            }
        }
        Ok(Theme {
            title,
            description: self.description.trim().to_owned(),
            appearance,
            vars,
        })
    }
}

fn value_of(var: StyleVar, path: &str, value: &str) -> Result<StyleValue, ThemeError> {
    StyleValue::parse(var.kind(), value).map_err(|reason| invalid(path.to_owned(), reason))
}

fn invalid(path: String, reason: String) -> ThemeError {
    ThemeError::InvalidValue { path, reason }
}

fn unknown(path: String, what: &'static str, known: &[StyleVar]) -> ThemeError {
    let known = known
        .iter()
        .map(|var| var.name())
        .collect::<Vec<_>>()
        .join(", ");
    ThemeError::UnknownVar { path, what, known }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(source: &str) -> String {
        Theme::from_json(source).unwrap_err().to_string()
    }

    fn value(theme: &Theme, var: StyleVar) -> &str {
        theme.get(var).map_or("", StyleValue::as_str)
    }

    #[test]
    fn every_builtin_theme_is_valid_and_complete() {
        let themes = builtin_themes();
        assert_eq!(themes.len(), BUILTIN.len());
        for (id, theme) in themes {
            assert!(super::super::profile::is_valid_name(id), "{id}");
            assert!(!theme.description.is_empty(), "{id}");
            for var in StyleVar::ALL {
                assert!(theme.get(var).is_some(), "{id} has no {var}");
            }
        }
        // The base themes set everything themselves.
        for appearance in Appearance::ALL {
            assert_eq!(base(appearance).vars.len(), StyleVar::ALL.len());
            assert_eq!(base(appearance).appearance, appearance);
        }
        let mut ids: Vec<&str> = BUILTIN.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), BUILTIN.len(), "duplicate built-in theme");
    }

    #[test]
    fn every_theme_file_in_the_folder_is_built_in() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/themes");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".json") && name != "theme.schema.json")
            .collect();
        files.sort();
        let mut listed: Vec<String> = BUILTIN.iter().map(|(id, _)| format!("{id}.json")).collect();
        listed.sort();
        assert_eq!(files, listed);
    }

    /// WCAG 2 contrast between two `#rrggbb` (or `#rgb`) colours.
    fn contrast(a: &str, b: &str) -> f64 {
        fn luminance(hex: &str) -> f64 {
            let hex = hex.strip_prefix('#').expect("a hex colour");
            let hex: String = match hex.len() {
                3 | 4 => hex.chars().take(3).flat_map(|c| [c, c]).collect(),
                _ => hex[..6].to_owned(),
            };
            let channel = |i: usize| {
                let v = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap()) / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4)
        }
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// Every label the app draws stays readable on what it is drawn on.
    #[test]
    fn builtin_themes_keep_labels_readable() {
        use StyleVar::*;
        let checks = [
            (Text, Bg, 7.0),
            (Text, Panel, 7.0),
            (Text, KeySpecial, 4.5),
            (KeyText, Key, 7.0),
            (KeyText, KeyPressed, 4.5),
            (KeySpecialText, KeySpecial, 4.5),
            (KeySpecialText, KeyPressed, 3.0),
            (Muted, Key, 4.5),
            (Muted, Bg, 4.5),
            (Muted, Panel, 4.5),
            (Muted, KeySpecial, 4.5),
            (Accent, KeySpecial, 4.5),
            (Accent, Panel, 4.5),
            (Accent, Bg, 3.0),
            (OnAccent, Accent, 4.5),
            (Warning, Panel, 4.5),
        ];
        let mut failures = Vec::new();
        for (id, theme) in builtin_themes() {
            for (fg, bg, needed) in checks {
                let ratio = contrast(value(theme, fg), value(theme, bg));
                if ratio < needed {
                    failures.push(format!("{id}: {fg} on {bg} is {ratio:.2}, needs {needed}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn parses_a_small_theme_and_fills_in_the_rest() {
        let theme = Theme::from_json(
            r##"{
                "$schema": "./theme.schema.json",
                "title": " Midnight ",
                "appearance": "dark",
                "colors": { "text": "#ffffff", "muted": "#aaaaaa", "accent": "rgb(255 180 84)" },
                "fonts": {
                    "ui": { "family": "Inter, sans-serif", "size": "14px", "weight": "bold" },
                    "keys": { "family": ["Fira Code", "monospace"], "weight": 500 }
                },
                "sizes": { "radius": "10px" }
            }"##,
        )
        .unwrap();
        assert_eq!(theme.title, "Midnight");
        assert_eq!(theme.appearance, Appearance::Dark);
        assert_eq!(value(&theme, StyleVar::Accent), "rgb(255 180 84)");
        assert_eq!(
            value(&theme, StyleVar::FontFamily),
            r#""Inter", sans-serif"#
        );
        assert_eq!(value(&theme, StyleVar::FontWeight), "bold");
        assert_eq!(
            value(&theme, StyleVar::KeyFontFamily),
            r#""Fira Code", monospace"#
        );
        assert_eq!(value(&theme, StyleVar::KeyFontWeight), "500");
        assert_eq!(value(&theme, StyleVar::Radius), "10px");
        // Key labels follow the theme's own text colours...
        assert_eq!(value(&theme, StyleVar::KeyText), "#ffffff");
        assert_eq!(value(&theme, StyleVar::KeySpecialText), "#aaaaaa");
        // ...and everything else comes from the built-in dark theme.
        let dark = base(Appearance::Dark);
        assert_eq!(theme.get(StyleVar::Bg), dark.get(StyleVar::Bg));
        assert_eq!(theme.get(StyleVar::Gap), dark.get(StyleVar::Gap));
        assert_eq!(
            theme.get(StyleVar::PreviewFontFamily),
            dark.get(StyleVar::PreviewFontFamily)
        );
        assert_eq!(theme.vars.len(), StyleVar::ALL.len());

        let light = Theme::from_json(r#"{ "title": "L", "appearance": "light" }"#).unwrap();
        assert_eq!(light.vars, base(Appearance::Light).vars);
    }

    #[test]
    fn reports_each_mistake() {
        assert!(error("{").contains("line 1"), "{}", error("{"));
        assert!(
            error(r#"{"title": "a", "appearance": "dark", "colours": {}}"#)
                .contains("unknown field `colours`")
        );
        assert!(error(r#"{"appearance": "dark"}"#).contains("missing field `title`"));
        assert_eq!(
            error(r#"{"title": " ", "appearance": "dark"}"#),
            "`title` must not be empty"
        );
        assert_eq!(
            error(r#"{"title": "a", "appearance": "dim"}"#),
            r#"`appearance` must be "light" or "dark", not "dim""#
        );
        assert!(
            error(r#"{"title": "a", "appearance": "dark", "colors": {"acent": "red"}}"#)
                .starts_with("`colors.acent` is not a theme colour (known: bg, panel, key,")
        );
        assert_eq!(
            error(r#"{"title": "a", "appearance": "dark", "colors": {"radius": "2px"}}"#)
                .split(" (")
                .next(),
            Some("`colors.radius` is not a theme colour")
        );
        assert_eq!(
            error(r#"{"title": "a", "appearance": "dark", "sizes": {"key-font-size": "2px"}}"#),
            "`sizes.key-font-size` is not a theme size (known: radius, gap)"
        );
        assert_eq!(
            error(r#"{"title": "a", "appearance": "dark", "colors": {"bg": "url(x)"}}"#),
            "`colors.bg`: function `url()` is not allowed"
        );
        assert_eq!(
            error(
                r#"{"title": "a", "appearance": "dark", "fonts": {"keys": {"family": ["\"x\""]}}}"#
            ),
            "`fonts.keys.family`: write font names without quotes, separated by commas"
        );
        assert_eq!(
            error(r#"{"title": "a", "appearance": "dark", "fonts": {"ui": {"weight": 2000}}}"#),
            "`fonts.ui.weight`: expected a font weight: normal, bold or a number from 1 to 1000"
        );
        assert!(
            error(r#"{"title": "a", "appearance": "dark", "fonts": {"ui": {"colour": "x"}}}"#)
                .contains("unknown field `colour`")
        );
        assert!(
            error(r#"{"title": "a", "appearance": "dark", "fonts": {"emoji": {}}}"#)
                .contains("unknown field `emoji`")
        );
    }

    #[test]
    fn builtin_sources_are_available_to_copy() {
        assert!(builtin_source("nord").unwrap().contains("\"Nord\""));
        assert_eq!(builtin_source("nope"), None);
    }

    /// `theme.schema.json` lists exactly the colours, sizes and font roles that
    /// theme files accept, so editors check theme files the way the app does.
    #[test]
    fn the_json_schema_matches_the_theme_format() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../themes/theme.schema.json")).unwrap();
        let keys = |pointer: &str| -> Vec<String> {
            let mut keys: Vec<String> = schema
                .pointer(pointer)
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("{pointer}"))
                .keys()
                .cloned()
                .collect();
            keys.sort();
            keys
        };
        let sorted = |names: Vec<&str>| {
            let mut names: Vec<String> = names.into_iter().map(str::to_owned).collect();
            names.sort();
            names
        };
        let colors = StyleVar::ALL
            .into_iter()
            .filter(|var| var.kind() == ValueKind::Color)
            .map(StyleVar::name)
            .collect();
        assert_eq!(keys("/properties/colors/properties"), sorted(colors));
        assert_eq!(
            keys("/properties/sizes/properties"),
            sorted(SIZES.map(StyleVar::name).to_vec())
        );
        assert_eq!(
            keys("/properties/fonts/properties"),
            sorted(FontRole::ALL.map(FontRole::name).to_vec())
        );
        assert_eq!(
            keys("/$defs/font/properties"),
            sorted(vec!["family", "size", "weight"])
        );
        assert_eq!(
            keys("/properties"),
            sorted(vec![
                "$schema",
                "title",
                "description",
                "appearance",
                "colors",
                "fonts",
                "sizes"
            ])
        );
    }
}
