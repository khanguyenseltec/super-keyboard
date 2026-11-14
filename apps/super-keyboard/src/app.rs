//! The root component.

use dioxus::prelude::*;
use super_keyboard_config::{Align, Settings};

use crate::{
    components::{KeyboardView, Notices, Preview, SettingsPanel, TopBar},
    state::{AppEnv, AppState, Panel},
    style, watch,
};

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let env = use_hook(|| try_consume_context::<AppEnv>().unwrap_or_else(AppEnv::detect));
    let state = use_context_provider(|| AppState::load(&env));
    watch::use_config_watcher(state, &env);

    let (css, frame) = {
        let config = state.config.read();
        (style::app_css(&config), Frame::new(&config.settings))
    };

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        div {
            class: "app",
            "data-theme": frame.theme,
            "data-width": frame.width,
            "data-align": frame.align,
            "data-fill": frame.fill,
            style { {css} }
            TopBar {}
            Notices {}
            match (state.panel)() {
                Panel::Preview => rsx! { Preview {} },
                Panel::Settings => rsx! { SettingsPanel {} },
            }
            KeyboardView {}
        }
    }
}

/// The attributes of `.app` that `assets/main.css` reads the theme and the
/// keyboard size from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    theme: String,
    /// Present when the keyboard has a width of its own.
    width: Option<&'static str>,
    /// Present when the keyboard is not centred.
    align: Option<&'static str>,
    /// `share` or `full` when the keyboard fills a share of the window.
    fill: Option<&'static str>,
}

impl Frame {
    fn new(settings: &Settings) -> Self {
        let keyboard = &settings.keyboard;
        Self {
            theme: settings.theme.choice.name().to_owned(),
            width: keyboard.width_pct.map(|_| "custom"),
            align: (keyboard.align != Align::Center).then(|| keyboard.align.name()),
            fill: keyboard.height_pct.map(|_| {
                if keyboard.fills_window() {
                    "full"
                } else {
                    "share"
                }
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use super_keyboard_config::ConfigDir;

    /// Renders the app without a webview, so the UI is checked on any CI host.
    fn render(env: AppEnv) -> String {
        let mut dom = VirtualDom::new(App).with_root_context(env);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn renders_the_base_layer() {
        let html = render(AppEnv::isolated());
        assert!(html.contains("Super Keyboard"), "{html}");
        assert!(html.contains(r#"<span class="label">q</span>"#), "{html}");
        assert!(html.contains(r#"aria-label="Layer base""#), "{html}");
        assert!(html.contains(r#"style="--rows:5;--cols:10""#), "{html}");
        assert!(html.contains(r#"data-theme="system""#), "{html}");
        assert!(!html.contains("data-fill"), "{html}");
        assert!(
            html.contains("@media (prefers-color-scheme:light)"),
            "{html}"
        );
        assert!(!html.contains("notices"), "{html}");
    }

    #[test]
    fn the_keyboard_size_reaches_the_page() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path());
        fs::write(
            dir.settings_file(),
            "theme = \"dracula\"\n[keyboard]\nwidth = 70\nheight = 100\nalign = \"right\"\n",
        )
        .unwrap();
        let html = render(AppEnv {
            config_dir: Some(dir),
            watch: false,
        });
        for attribute in [
            r#"data-theme="dracula""#,
            r#"data-width="custom""#,
            r#"data-align="right""#,
            r#"data-fill="full""#,
        ] {
            assert!(html.contains(attribute), "{attribute}: {html}");
        }
        assert!(html.contains("--keyboard-width:70%;"), "{html}");
        assert!(html.contains("--keyboard-height:100%;"), "{html}");
        assert!(html.contains("--bg:#282a36;"), "{html}");
        assert!(!html.contains("prefers-color-scheme"), "{html}");
    }

    #[test]
    fn frames_follow_the_settings() {
        let frame = |toml: &str| Frame::new(&Settings::from_toml(toml).unwrap());
        assert_eq!(
            frame(""),
            Frame {
                theme: "system".into(),
                width: None,
                align: None,
                fill: None,
            }
        );
        let share = frame("[keyboard]\nheight = 40\nalign = \"left\"");
        assert_eq!((share.fill, share.align), (Some("share"), Some("left")));
    }

    #[test]
    fn renders_user_layouts_settings_and_problems() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path());
        fs::create_dir_all(dir.layouts_dir()).unwrap();
        fs::write(
            dir.layouts_dir().join("mine.toml"),
            "name = \"mine\"\n[[layers]]\nname = \"home\"\nrows = [[\"zz\"]]\n",
        )
        .unwrap();
        fs::write(dir.layouts_dir().join("broken.toml"), "name = ").unwrap();
        fs::write(
            dir.settings_file(),
            "profile = \"mine\"\ntheme = \"light\"\n[style.light]\naccent = \"#c2410c\"\n",
        )
        .unwrap();

        let html = render(AppEnv {
            config_dir: Some(dir),
            watch: false,
        });
        assert!(html.contains(r#"data-theme="light""#), "{html}");
        assert!(html.contains(r#"<span class="label">zz</span>"#), "{html}");
        assert!(html.contains("--accent:#c2410c;"), "{html}");
        assert!(html.contains(r#"<option value="mine""#), "{html}");
        assert!(html.contains(r#"<option value="terminal""#), "{html}");
        assert!(html.contains("broken.toml"), "{html}");
    }

    #[test]
    fn profiles_arrange_the_rendered_keyboard() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path());
        fs::write(dir.settings_file(), "profile = \"terminal\"\n").unwrap();
        let html = render(AppEnv {
            config_dir: Some(dir),
            watch: false,
        });
        // The terminal row is on; the bracket row is off.
        assert!(html.contains(r#"<span class="label">~</span>"#), "{html}");
        assert!(!html.contains(r#"<span class="label">(</span>"#), "{html}");
    }
}
