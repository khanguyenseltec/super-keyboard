//! Turns settings into the CSS that overrides `assets/main.css`.
//!
//! Values were validated by `super_keyboard_config::style`, so they can be
//! written into the stylesheet as they are.

use std::{collections::BTreeMap, fmt::Write};

use super_keyboard_config::{Settings, StyleValue, StyleVar};

/// The user's overrides, or an empty string when there are none.
///
/// Selectors carry the `data-theme` attribute so they outrank the built-in
/// theme rules, and per-theme rules come last so they win over rules for both.
pub fn user_css(settings: &Settings) -> String {
    let mut css = String::new();
    let style = &settings.style;

    let mut both = declarations(&style.both);
    if let Some(px) = settings.keys.height_px {
        let _ = write!(both, "--key-height:{px}px;");
    }
    rule(&mut css, ".app[data-theme]", &both);

    for (theme, vars) in [("dark", &style.dark), ("light", &style.light)] {
        let body = declarations(vars);
        rule(&mut css, &format!(".app[data-theme={theme}]"), &body);
        if !body.is_empty() {
            let _ = write!(
                css,
                "@media (prefers-color-scheme:{theme}){{.app[data-theme=system]{{{body}}}}}"
            );
        }
    }
    css
}

fn declarations(vars: &BTreeMap<StyleVar, StyleValue>) -> String {
    vars.iter().fold(String::new(), |mut out, (var, value)| {
        let _ = write!(out, "--{var}:{value};");
        out
    })
}

fn rule(css: &mut String, selector: &str, body: &str) {
    if !body.is_empty() {
        let _ = write!(css, "{selector}{{{body}}}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_overrides_no_css() {
        assert_eq!(user_css(&Settings::default()), "");
    }

    #[test]
    fn writes_rules_for_both_themes_then_each_theme() {
        let settings = Settings::from_toml(
            r##"
            [keys]
            height = 52
            [style]
            radius = "10px"
            [style.dark]
            accent = "#ff9e64"
            "##,
        )
        .unwrap();
        assert_eq!(
            user_css(&settings),
            ".app[data-theme]{--radius:10px;--key-height:52px;}\
             .app[data-theme=dark]{--accent:#ff9e64;}\
             @media (prefers-color-scheme:dark){.app[data-theme=system]{--accent:#ff9e64;}}"
        );
    }
}
