//! Turns the active theme and settings into the CSS that `assets/main.css`
//! reads its variables from.
//!
//! Values were validated by `super_keyboard_config` (themes and `[style]`
//! alike), so they can be written into the stylesheet as they are.

use std::{collections::BTreeMap, fmt::Write};

use super_keyboard_config::{
    ActiveTheme, Appearance, Settings, Style, StyleValue, StyleVar, Theme, UserConfig,
};

/// The rule the variables go in. The attribute outranks `.app` rules in
/// `main.css`, such as the key height for phones in landscape.
const SELECTOR: &str = ".app[data-theme]";

/// The active theme with the user's `[style]` changes, and the sizes from
/// settings. Following the system writes the dark theme, then the light theme
/// for when the system is light.
pub fn app_css(config: &UserConfig) -> String {
    let settings = &config.settings;
    let mut css = String::new();
    let (main, light) = match config.active_theme() {
        ActiveTheme::Fixed(entry) => (&entry.theme, None),
        ActiveTheme::System { light, dark } => (&dark.theme, Some(&light.theme)),
    };
    let mut body = theme_declarations(main, &settings.style);
    body.push_str(&size_declarations(settings));
    let _ = write!(css, "{SELECTOR}{{{body}}}");
    if let Some(light) = light {
        let body = theme_declarations(light, &settings.style);
        let _ = write!(
            css,
            "@media (prefers-color-scheme:light){{{SELECTOR}{{{body}}}}}"
        );
    }
    css
}

/// A theme's variables as an inline style, to preview it in the theme list.
pub fn theme_preview_style(theme: &Theme) -> String {
    declarations(&theme.vars)
}

/// A theme's variables with the user's changes for every theme and for themes
/// of its appearance, which win, and its colour scheme for form controls.
fn theme_declarations(theme: &Theme, style: &Style) -> String {
    let own = match theme.appearance {
        Appearance::Light => &style.light,
        Appearance::Dark => &style.dark,
    };
    let mut vars = theme.vars.clone();
    vars.extend(style.both.iter().map(|(k, v)| (*k, v.clone())));
    vars.extend(own.iter().map(|(k, v)| (*k, v.clone())));
    let mut out = declarations(&vars);
    let _ = write!(out, "color-scheme:{};", theme.appearance);
    out
}

/// Key height and keyboard size, where settings fix them.
fn size_declarations(settings: &Settings) -> String {
    let mut out = String::new();
    if let Some(px) = settings.keys.height_px {
        let _ = write!(out, "--key-height:{px}px;");
    }
    if let Some(pct) = settings.keyboard.width_pct {
        let _ = write!(out, "--keyboard-width:{pct}%;");
    }
    if let Some(pct) = settings.keyboard.height_pct {
        let _ = write!(out, "--keyboard-height:{pct}%;");
    }
    out
}

fn declarations(vars: &BTreeMap<StyleVar, StyleValue>) -> String {
    vars.iter().fold(String::new(), |mut out, (var, value)| {
        let _ = write!(out, "--{var}:{value};");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn css(settings: &str) -> String {
        let config = UserConfig {
            settings: Settings::from_toml(settings).unwrap(),
            ..UserConfig::default()
        };
        app_css(&config)
    }

    /// The value of `--name` in the first rule that sets it.
    fn var<'a>(css: &'a str, name: &str) -> Option<&'a str> {
        let start = css.find(&format!("--{name}:"))? + name.len() + 3;
        Some(&css[start..start + css[start..].find(';')?])
    }

    #[test]
    fn follows_the_system_by_default() {
        let css = css("");
        let light = css.find("@media (prefers-color-scheme:light){").unwrap();
        let (dark, light) = css.split_at(light);
        assert!(dark.starts_with(".app[data-theme]{--bg:#0f1115;"), "{dark}");
        assert!(dark.contains("color-scheme:dark;"), "{dark}");
        assert_eq!(var(light, "bg"), Some("#f4f5f7"));
        assert!(light.contains("color-scheme:light;"), "{light}");
        assert_eq!(
            var(dark, "key-font-family"),
            Some(
                r#""JetBrains Mono", "Fira Code", "Ubuntu Mono", "DejaVu Sans Mono", ui-monospace, monospace"#
            )
        );
    }

    #[test]
    fn a_named_theme_and_the_users_changes() {
        let css = css(r##"
            theme = "nord"
            [keys]
            height = 52
            [keyboard]
            width = 80
            height = 45
            [style]
            radius = "10px"
            accent = "red"
            [style.dark]
            accent = "#ff9e64"
            [style.light]
            gap = "1px"
            "##);
        assert!(!css.contains("@media"), "{css}");
        assert_eq!(var(&css, "bg"), Some("#2e3440"));
        assert_eq!(var(&css, "radius"), Some("10px"));
        assert_eq!(var(&css, "accent"), Some("#ff9e64"), "dark changes win");
        assert_eq!(var(&css, "gap"), Some("4px"), "light changes do not apply");
        assert_eq!(var(&css, "key-height"), Some("52px"));
        assert_eq!(var(&css, "keyboard-width"), Some("80%"));
        assert_eq!(var(&css, "keyboard-height"), Some("45%"));
        assert_eq!(css.matches("--accent:").count(), 1, "{css}");
    }

    #[test]
    fn system_uses_the_chosen_light_and_dark_themes() {
        let css = css(
            "light-theme = \"paper\"\ndark-theme = \"dracula\"\n[style.light]\naccent = \"#c2410c\"",
        );
        let (dark, light) = css.split_at(css.find("@media").unwrap());
        assert_eq!(var(dark, "bg"), Some("#282a36"));
        assert_eq!(var(light, "bg"), Some("#f4ecd8"));
        assert_eq!(var(light, "accent"), Some("#c2410c"));
        assert_eq!(var(dark, "accent"), Some("#bd93f9"));
    }

    #[test]
    fn previews_show_the_theme_as_it_is() {
        let config = UserConfig::default();
        let paper = &config.themes.get("paper").unwrap().theme;
        let style = theme_preview_style(paper);
        assert_eq!(var(&style, "bg"), Some("#f4ecd8"));
        assert_eq!(style.matches("--").count(), StyleVar::ALL.len());
    }
}
