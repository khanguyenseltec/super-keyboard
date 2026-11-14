//! The root component.

use dioxus::prelude::*;

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

    let (theme, user_css) = {
        let config = state.config.read();
        (
            config.settings.theme.name(),
            style::user_css(&config.settings),
        )
    };

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        div { class: "app", "data-theme": theme,
            if !user_css.is_empty() {
                style { {user_css} }
            }
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
        assert!(html.contains(r#"data-theme="system""#), "{html}");
        assert!(!html.contains("notices"), "{html}");
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
