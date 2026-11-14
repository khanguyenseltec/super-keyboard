//! Colour and size overrides from the `[style]` tables of `settings.toml`.
//!
//! ```toml
//! [style]            # both themes
//! radius = "10px"
//!
//! [style.dark]
//! accent = "#ff9e64"
//!
//! [style.light]
//! accent = "oklch(55% 0.15 45)"
//! ```
//!
//! Values are CSS, but only the safe part of it: colours, lengths and the maths
//! functions that combine them. Anything that could load a resource (`url(…)`,
//! `image-set(…)`, `@import`) or escape the declaration (quotes, `;`, braces,
//! backslash escapes) is rejected, because a keyboard must never be able to send
//! what it shows anywhere. See `docs/CUSTOMIZING.md` for every variable.

use std::{collections::BTreeMap, fmt};

/// A style variable a user can override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StyleVar {
    Bg,
    Panel,
    Key,
    KeySpecial,
    KeyPressed,
    Text,
    Muted,
    Accent,
    OnAccent,
    Selection,
    Warning,
    Radius,
    Gap,
    KeyFont,
}

/// What kind of CSS value a variable takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Color,
    Length,
}

impl StyleVar {
    pub const ALL: [Self; 14] = [
        Self::Bg,
        Self::Panel,
        Self::Key,
        Self::KeySpecial,
        Self::KeyPressed,
        Self::Text,
        Self::Muted,
        Self::Accent,
        Self::OnAccent,
        Self::Selection,
        Self::Warning,
        Self::Radius,
        Self::Gap,
        Self::KeyFont,
    ];

    /// The name used in `settings.toml`; the CSS variable is `--<name>`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bg => "bg",
            Self::Panel => "panel",
            Self::Key => "key",
            Self::KeySpecial => "key-special",
            Self::KeyPressed => "key-pressed",
            Self::Text => "text",
            Self::Muted => "muted",
            Self::Accent => "accent",
            Self::OnAccent => "on-accent",
            Self::Selection => "selection",
            Self::Warning => "warning",
            Self::Radius => "radius",
            Self::Gap => "gap",
            Self::KeyFont => "key-font",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|var| var.name() == name)
    }

    pub fn kind(self) -> ValueKind {
        match self {
            Self::Radius | Self::Gap | Self::KeyFont => ValueKind::Length,
            _ => ValueKind::Color,
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
            Self::Text => "main text and key labels",
            Self::Muted => "secondary text and named-key labels",
            Self::Accent => "caret, one-shot modifiers and the active layer",
            Self::OnAccent => "label of a locked modifier, drawn on the accent colour",
            Self::Selection => "selected text in the preview",
            Self::Warning => "notices about layout and settings errors",
            Self::Radius => "key corner radius",
            Self::Gap => "space between keys",
            Self::KeyFont => "key label size",
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

impl StyleValue {
    /// Checks `value` for a variable of kind `kind`. The error says why it was
    /// rejected.
    pub fn parse(kind: ValueKind, value: &str) -> Result<Self, String> {
        let value = value.trim();
        if value.is_empty() {
            return Err("empty value".into());
        }
        if value.len() > MAX_VALUE_LEN {
            return Err(format!("longer than {MAX_VALUE_LEN} characters"));
        }
        if let Some(c) = value.chars().find(|c| !is_allowed_char(*c)) {
            return Err(format!("`{c}` is not allowed in a style value"));
        }
        check_functions(value)?;
        let shape_ok = match kind {
            ValueKind::Color => is_color(value),
            ValueKind::Length => is_length(value),
        };
        if !shape_ok {
            return Err(match kind {
                ValueKind::Color => "expected a colour such as #7aa2f7, rgb(…) or a colour name",
                ValueKind::Length => "expected a length such as 6px, 0.5rem or calc(…)",
            }
            .into());
        }
        Ok(Self(value.to_owned()))
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

/// Style overrides: for both themes, and for each theme on its own (which win).
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

    #[test]
    fn variable_names_round_trip() {
        for var in StyleVar::ALL {
            assert_eq!(StyleVar::from_name(var.name()), Some(var));
            assert!(!var.description().is_empty());
        }
        assert_eq!(StyleVar::from_name("nope"), None);
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
}
