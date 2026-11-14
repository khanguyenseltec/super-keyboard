//! Colour, font and size overrides from the `[style]` tables of `settings.toml`,
//! and the values themes are made of (see [`crate::theme`]).
//!
//! ```toml
//! [style]            # every theme
//! radius = "10px"
//! key-font-family = "Fira Code, monospace"
//!
//! [style.dark]       # dark themes only
//! accent = "#ff9e64"
//!
//! [style.light]      # light themes only
//! accent = "oklch(55% 0.15 45)"
//! ```
//!
//! Values are CSS, but only the safe part of it: colours, lengths, the maths
//! functions that combine them, font weights, and the names of installed fonts.
//! Anything that could load a resource (`url(…)`, `image-set(…)`, `@import`,
//! `@font-face`) or escape the declaration (quotes, `;`, braces, backslash
//! escapes) is rejected, because a keyboard must never be able to send what it
//! shows anywhere. See `docs/CUSTOMIZING.md` for every variable.

use std::{collections::BTreeMap, fmt};

/// A style variable a theme or the user can set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StyleVar {
    Bg,
    Panel,
    Key,
    KeySpecial,
    KeyPressed,
    KeyBorder,
    Text,
    KeyText,
    KeySpecialText,
    Muted,
    Accent,
    OnAccent,
    Selection,
    Warning,
    Radius,
    Gap,
    FontFamily,
    FontSize,
    FontWeight,
    KeyFontFamily,
    KeyFontSize,
    KeyFontWeight,
    PreviewFontFamily,
    PreviewFontSize,
    PreviewFontWeight,
}

/// What kind of CSS value a variable takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Color,
    Length,
    /// Installed fonts and generic families, such as `Fira Code, monospace`.
    FontFamily,
    /// `normal`, `bold`, `lighter`, `bolder` or a number from 1 to 1000.
    FontWeight,
}

/// Which text a font variable styles, as grouped under `fonts` in theme files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontRole {
    /// The top bar, settings and notices.
    Ui,
    /// Key labels.
    Keys,
    /// The typed text in the preview.
    Preview,
}

impl FontRole {
    pub const ALL: [Self; 3] = [Self::Ui, Self::Keys, Self::Preview];

    /// The name under `fonts` in theme files.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ui => "ui",
            Self::Keys => "keys",
            Self::Preview => "preview",
        }
    }

    /// The family, size and weight variables of this role.
    pub fn vars(self) -> [StyleVar; 3] {
        match self {
            Self::Ui => [
                StyleVar::FontFamily,
                StyleVar::FontSize,
                StyleVar::FontWeight,
            ],
            Self::Keys => [
                StyleVar::KeyFontFamily,
                StyleVar::KeyFontSize,
                StyleVar::KeyFontWeight,
            ],
            Self::Preview => [
                StyleVar::PreviewFontFamily,
                StyleVar::PreviewFontSize,
                StyleVar::PreviewFontWeight,
            ],
        }
    }
}

impl StyleVar {
    pub const ALL: [Self; 25] = [
        Self::Bg,
        Self::Panel,
        Self::Key,
        Self::KeySpecial,
        Self::KeyPressed,
        Self::KeyBorder,
        Self::Text,
        Self::KeyText,
        Self::KeySpecialText,
        Self::Muted,
        Self::Accent,
        Self::OnAccent,
        Self::Selection,
        Self::Warning,
        Self::Radius,
        Self::Gap,
        Self::FontFamily,
        Self::FontSize,
        Self::FontWeight,
        Self::KeyFontFamily,
        Self::KeyFontSize,
        Self::KeyFontWeight,
        Self::PreviewFontFamily,
        Self::PreviewFontSize,
        Self::PreviewFontWeight,
    ];

    /// The name used in `settings.toml` and theme files; the CSS variable is
    /// `--<name>`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bg => "bg",
            Self::Panel => "panel",
            Self::Key => "key",
            Self::KeySpecial => "key-special",
            Self::KeyPressed => "key-pressed",
            Self::KeyBorder => "key-border",
            Self::Text => "text",
            Self::KeyText => "key-text",
            Self::KeySpecialText => "key-special-text",
            Self::Muted => "muted",
            Self::Accent => "accent",
            Self::OnAccent => "on-accent",
            Self::Selection => "selection",
            Self::Warning => "warning",
            Self::Radius => "radius",
            Self::Gap => "gap",
            Self::FontFamily => "font-family",
            Self::FontSize => "font-size",
            Self::FontWeight => "font-weight",
            Self::KeyFontFamily => "key-font-family",
            Self::KeyFontSize => "key-font-size",
            Self::KeyFontWeight => "key-font-weight",
            Self::PreviewFontFamily => "preview-font-family",
            Self::PreviewFontSize => "preview-font-size",
            Self::PreviewFontWeight => "preview-font-weight",
        }
    }

    /// Looks a variable up by name. `key-font`, its name before fonts could be
    /// themed, still means `key-font-size`.
    pub fn from_name(name: &str) -> Option<Self> {
        if name == "key-font" {
            return Some(Self::KeyFontSize);
        }
        Self::ALL.into_iter().find(|var| var.name() == name)
    }

    pub fn kind(self) -> ValueKind {
        match self {
            Self::Radius
            | Self::Gap
            | Self::FontSize
            | Self::KeyFontSize
            | Self::PreviewFontSize => ValueKind::Length,
            Self::FontFamily | Self::KeyFontFamily | Self::PreviewFontFamily => {
                ValueKind::FontFamily
            }
            Self::FontWeight | Self::KeyFontWeight | Self::PreviewFontWeight => {
                ValueKind::FontWeight
            }
            _ => ValueKind::Color,
        }
    }

    /// The variable a theme's value falls back to when the theme leaves this
    /// one out, so key labels follow the theme's own text colours.
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::KeyText => Some(Self::Text),
            Self::KeySpecialText => Some(Self::Muted),
            _ => None,
        }
    }

    /// What the variable colours or sizes.
    pub fn description(self) -> &'static str {
        match self {
            Self::Bg => "app background",
            Self::Panel => "top bar and keyboard background",
            Self::Key => "key background",
            Self::KeySpecial => "background of Esc, modifier, layer and other named keys",
            Self::KeyPressed => "key background while pressed",
            Self::KeyBorder => "key outline (transparent for none)",
            Self::Text => "main text",
            Self::KeyText => "key labels (default: text)",
            Self::KeySpecialText => "labels of named keys (default: muted)",
            Self::Muted => "secondary text and hold hints",
            Self::Accent => "caret, one-shot modifiers and the active layer",
            Self::OnAccent => "label of a locked modifier, drawn on the accent colour",
            Self::Selection => "selected text in the preview",
            Self::Warning => "notices about layout and settings errors",
            Self::Radius => "key corner radius",
            Self::Gap => "space between keys",
            Self::FontFamily => "fonts of the top bar, settings and notices",
            Self::FontSize => "text size of the top bar, settings and notices",
            Self::FontWeight => "text weight of the top bar, settings and notices",
            Self::KeyFontFamily => "fonts of key labels",
            Self::KeyFontSize => "key label size",
            Self::KeyFontWeight => "key label weight",
            Self::PreviewFontFamily => "fonts of the typed text in the preview",
            Self::PreviewFontSize => "size of the typed text in the preview",
            Self::PreviewFontWeight => "weight of the typed text in the preview",
        }
    }
}

impl fmt::Display for StyleVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A CSS value that has passed validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyleValue(String);

/// Functions a value may call. None of them can load anything.
const ALLOWED_FUNCTIONS: [&str; 15] = [
    "rgb",
    "rgba",
    "hsl",
    "hsla",
    "hwb",
    "lab",
    "lch",
    "oklab",
    "oklch",
    "color-mix",
    "calc",
    "min",
    "max",
    "clamp",
    "var",
];

const LENGTH_UNITS: [&str; 12] = [
    "px", "em", "rem", "%", "vh", "vw", "vmin", "vmax", "dvh", "svh", "lvh", "ch",
];

const MAX_VALUE_LEN: usize = 80;

/// Font family keywords, written without quotes; every other name is quoted.
const GENERIC_FAMILIES: [&str; 13] = [
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "emoji",
    "math",
    "fangsong",
];

const MAX_FONT_NAME_CHARS: usize = 64;
const MAX_FONTS: usize = 12;

impl StyleValue {
    /// Checks `value` for a variable of kind `kind`. The error says why it was
    /// rejected. Font families are font names separated by commas.
    pub fn parse(kind: ValueKind, value: &str) -> Result<Self, String> {
        let value = value.trim();
        if value.is_empty() {
            return Err("empty value".into());
        }
        let shape: fn(&str) -> bool = match kind {
            ValueKind::FontFamily => return Self::font_family(value.split(',')),
            ValueKind::FontWeight => return font_weight(value),
            ValueKind::Color => is_color,
            ValueKind::Length => is_length,
        };
        if value.len() > MAX_VALUE_LEN {
            return Err(format!("longer than {MAX_VALUE_LEN} characters"));
        }
        if let Some(c) = value.chars().find(|c| !is_allowed_char(*c)) {
            return Err(format!("`{c}` is not allowed in a style value"));
        }
        check_functions(value)?;
        if !shape(value) {
            return Err(match kind {
                ValueKind::Color => "expected a colour such as #7aa2f7, rgb(…) or a colour name",
                _ => "expected a length such as 6px, 0.5rem or calc(…)",
            }
            .into());
        }
        Ok(Self(value.to_owned()))
    }

    /// A font family from a list of font names, as theme files write it.
    ///
    /// Names may use letters and digits in any script, spaces and `._+&-`. Each
    /// becomes a quoted CSS string and generic families stay bare, so no name
    /// can close the string or the declaration. Fonts are looked up among the
    /// installed ones, never downloaded.
    pub fn font_family<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<Self, String> {
        let mut css = Vec::new();
        for name in names {
            let name = name.trim();
            if name.is_empty() {
                return Err("empty font name".into());
            }
            if name.chars().count() > MAX_FONT_NAME_CHARS {
                return Err(format!(
                    "font name longer than {MAX_FONT_NAME_CHARS} characters"
                ));
            }
            let allowed =
                |c: char| c.is_alphanumeric() || matches!(c, ' ' | '.' | '_' | '+' | '&' | '-');
            if let Some(c) = name.chars().find(|c| !allowed(*c)) {
                return Err(if matches!(c, '"' | '\'') {
                    "write font names without quotes, separated by commas".into()
                } else {
                    format!("`{c}` is not allowed in a font name")
                });
            }
            let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
            let lower = name.to_ascii_lowercase();
            css.push(if GENERIC_FAMILIES.contains(&lower.as_str()) {
                lower
            } else {
                format!("\"{name}\"")
            });
        }
        match css.len() {
            0 => Err("empty value".into()),
            n if n > MAX_FONTS => Err(format!("more than {MAX_FONTS} fonts")),
            _ => Ok(Self(css.join(", "))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StyleValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn font_weight(value: &str) -> Result<StyleValue, String> {
    let ok = matches!(value, "normal" | "bold" | "lighter" | "bolder")
        || value.parse::<u16>().is_ok_and(|n| (1..=1000).contains(&n));
    if ok {
        Ok(StyleValue(value.to_owned()))
    } else {
        Err("expected a font weight: normal, bold or a number from 1 to 1000".into())
    }
}

fn is_allowed_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '#' | '%' | '.' | ',' | '(' | ')' | '/' | ' ' | '+' | '-')
}

/// Every `name(` must be an allowed function, and parentheses must balance.
fn check_functions(value: &str) -> Result<(), String> {
    let mut depth = 0usize;
    let mut ident_start = None;
    for (i, c) in value.char_indices() {
        match c {
            'a'..='z' | 'A'..='Z' | '-' => {
                ident_start.get_or_insert(i);
            }
            '(' => {
                let name = ident_start.map_or("", |start| &value[start..i]);
                let name = name.trim_start_matches('-');
                if !ALLOWED_FUNCTIONS.contains(&name.to_ascii_lowercase().as_str()) {
                    return Err(if name.is_empty() {
                        "`(` must follow a function name".into()
                    } else {
                        format!("function `{name}()` is not allowed")
                    });
                }
                depth += 1;
                ident_start = None;
            }
            ')' => {
                depth = depth.checked_sub(1).ok_or("unbalanced `)`")?;
                ident_start = None;
            }
            _ => ident_start = None,
        }
    }
    if depth == 0 {
        Ok(())
    } else {
        Err("unclosed `(`".into())
    }
}

fn is_function_call(value: &str) -> bool {
    value.ends_with(')') && value.contains('(')
}

fn is_color(value: &str) -> bool {
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    // Named colours (`rebeccapurple`, `transparent`) or a colour function.
    value.chars().all(|c| c.is_ascii_alphabetic()) || is_function_call(value)
}

fn is_length(value: &str) -> bool {
    if is_function_call(value) {
        return true;
    }
    let number_end = value
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(number_end);
    let is_number = number.parse::<f64>().is_ok_and(f64::is_finite);
    is_number && (LENGTH_UNITS.contains(&unit) || (unit.is_empty() && number.parse() == Ok(0.0)))
}

/// Style overrides: for every theme, and for light or dark themes only (which
/// win).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub both: BTreeMap<StyleVar, StyleValue>,
    pub dark: BTreeMap<StyleVar, StyleValue>,
    pub light: BTreeMap<StyleVar, StyleValue>,
}

impl Style {
    pub fn is_empty(&self) -> bool {
        self.both.is_empty() && self.dark.is_empty() && self.light.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(value: &str) -> Result<StyleValue, String> {
        StyleValue::parse(ValueKind::Color, value)
    }

    fn length(value: &str) -> Result<StyleValue, String> {
        StyleValue::parse(ValueKind::Length, value)
    }

    fn family(value: &str) -> Result<String, String> {
        StyleValue::parse(ValueKind::FontFamily, value).map(|v| v.as_str().to_owned())
    }

    #[test]
    fn variable_names_round_trip() {
        for var in StyleVar::ALL {
            assert_eq!(StyleVar::from_name(var.name()), Some(var));
            assert!(!var.description().is_empty());
        }
        assert_eq!(StyleVar::from_name("nope"), None);
        // The name before fonts could be themed.
        assert_eq!(StyleVar::from_name("key-font"), Some(StyleVar::KeyFontSize));
    }

    #[test]
    fn every_font_role_has_a_family_size_and_weight() {
        for role in FontRole::ALL {
            assert_eq!(
                role.vars().map(StyleVar::kind),
                [
                    ValueKind::FontFamily,
                    ValueKind::Length,
                    ValueKind::FontWeight
                ]
            );
        }
    }

    #[test]
    fn accepts_colours() {
        for value in [
            "#fff",
            "#7aa2f7",
            "#7aa2f780",
            "rebeccapurple",
            "transparent",
            "rgb(10 20 30 / 50%)",
            "rgba(10, 20, 30, 0.5)",
            "oklch(55% 0.15 45)",
            "color-mix(in srgb, #fff 20%, var(--accent))",
            "  #abc  ",
        ] {
            assert!(color(value).is_ok(), "{value}: {:?}", color(value));
        }
        assert_eq!(color("  #abc ").unwrap().as_str(), "#abc");
    }

    #[test]
    fn accepts_lengths() {
        for value in [
            "6px",
            "0.5rem",
            "0",
            "12.5%",
            "-1px",
            "calc(1rem + 2px)",
            "clamp(12px, 2vw, 18px)",
        ] {
            assert!(length(value).is_ok(), "{value}: {:?}", length(value));
        }
    }

    #[test]
    fn rejects_values_that_could_load_or_escape() {
        for value in [
            "url(https://example.com/x.png)",
            "URL(x)",
            "image-set(x 1x)",
            "-webkit-image-set(x 1x)",
            "red; background: blue",
            "red } body { color: red",
            "\"quoted\"",
            "u\\72l(x)",
            "@import x",
            "expression(alert(1))",
            "(1px)",
            "rgb(1, 2, 3",
            "rgb(1, 2, 3))",
            "",
        ] {
            assert!(color(value).is_err(), "accepted {value:?}");
            assert!(length(value).is_err(), "accepted {value:?}");
            assert!(family(value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn rejects_the_wrong_kind() {
        assert!(color("12px").is_err());
        assert!(color("#12345").is_err());
        assert!(length("red").is_err());
        assert!(length("12").is_err());
        assert!(length("12parsecs").is_err());
    }

    #[test]
    fn font_names_are_quoted_and_generic_families_are_not() {
        assert_eq!(
            family("Fira Code,  JetBrains   Mono , MONOSPACE").unwrap(),
            r#""Fira Code", "JetBrains Mono", monospace"#
        );
        assert_eq!(family("Noto Sans CJK JP").unwrap(), r#""Noto Sans CJK JP""#);
        assert_eq!(family("源ノ角ゴシック").unwrap(), r#""源ノ角ゴシック""#);
        assert_eq!(
            StyleValue::font_family(["M+ 1m", "ui-monospace"])
                .unwrap()
                .as_str(),
            r#""M+ 1m", ui-monospace"#
        );
    }

    #[test]
    fn font_names_cannot_escape_or_load() {
        assert_eq!(
            family(r#""Fira Code", monospace"#).unwrap_err(),
            "write font names without quotes, separated by commas"
        );
        for value in [
            "Fira; color: red",
            "a} body {",
            "local(Fira)",
            "x\\22",
            "a</style>",
            "Fira,,mono",
            "Fira,",
        ] {
            assert!(family(value).is_err(), "accepted {value:?}");
        }
        assert!(family(&["a"; 13].join(",")).is_err());
        assert!(family(&"a".repeat(65)).is_err());
    }

    #[test]
    fn font_weights() {
        let weight = |v| StyleValue::parse(ValueKind::FontWeight, v);
        for value in ["normal", "bold", "bolder", "lighter", "1", "450", "1000"] {
            assert!(weight(value).is_ok(), "{value}");
        }
        for value in ["0", "1001", "heavy", "500px", "-1", "calc(400)"] {
            assert!(weight(value).is_err(), "accepted {value}");
        }
    }
}
