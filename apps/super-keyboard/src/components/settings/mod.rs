//! The settings panel: tabs for general settings, Vietnamese typing, themes,
//! the sections of the active profile, and the profiles and their menu.
//!
//! Changes apply at once and are saved to `settings.toml`, keeping the file's
//! comments. Styles, themes and layouts are edited as files; the panel says
//! where.

mod fields;
mod general;
mod profiles;
mod sections;
mod themes;
mod vietnamese;

use dioxus::prelude::*;

use crate::state::AppState;
use general::GeneralSettings;
use profiles::ProfilesEditor;
use sections::SectionsEditor;
use themes::ThemesEditor;
use vietnamese::VietnameseSettings;

/// A tab of the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    General,
    Vietnamese,
    Themes,
    Sections,
    Profiles,
}

impl Tab {
    const ALL: [Self; 5] = [
        Self::General,
        Self::Vietnamese,
        Self::Themes,
        Self::Sections,
        Self::Profiles,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Vietnamese => "Tiếng Việt",
            Self::Themes => "Themes",
            Self::Sections => "Sections",
            Self::Profiles => "Profiles",
        }
    }
}

#[component]
pub fn SettingsPanel() -> Element {
    let state = use_context::<AppState>();
    let mut tab = use_signal(|| Tab::General);
    let (settings_file, layouts_dir) = {
        let config = state.config.read();
        let dir = config.dir.as_ref();
        (
            dir.map(|d| d.settings_file().display().to_string()),
            dir.map(|d| d.layouts_dir().display().to_string()),
        )
    };
    let save_error = state.save_error.read().clone();

    rsx! {
        section { class: "settings", aria_label: "Settings",
            nav { class: "tabs", role: "tablist",
                for t in Tab::ALL {
                    button {
                        key: "{t.label()}",
                        class: "tab",
                        role: "tab",
                        aria_selected: tab() == t,
                        onclick: move |_| tab.set(t),
                        "{t.label()}"
                    }
                }
            }
            match tab() {
                Tab::General => rsx! { GeneralSettings {} },
                Tab::Vietnamese => rsx! { VietnameseSettings {} },
                Tab::Themes => rsx! { ThemesEditor {} },
                Tab::Sections => rsx! { SectionsEditor {} },
                Tab::Profiles => rsx! { ProfilesEditor {} },
            }
            footer { class: "settings-footer",
                if let Some(error) = save_error {
                    p { class: "save-status error", role: "alert", "{error}" }
                } else if let Some(file) = settings_file {
                    p { class: "save-status", "Saved to " code { "{file}" } }
                }
                if let Some(dir) = layouts_dir {
                    p { class: "save-status",
                        "Add layouts as " code { "*.toml" } " files in " code { "{dir}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppEnv;

    fn render(root: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(root);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn the_sections_tab_lists_every_section_in_order() {
        let html = render(|| {
            use_context_provider(|| AppState::load(&AppEnv::isolated()));
            rsx! { SectionsEditor {} }
        });
        let number_row = html.find("Number row").expect("number row");
        let brackets = html.find("Esc, Tab, brackets").expect("brackets");
        let bottom = html.find("Space bar, modifiers").expect("bottom");
        assert!(number_row < brackets && brackets < bottom, "{html}");
        assert!(html.contains("always on"), "{html}");
        assert!(
            html.contains(r#"aria-label="Move Number row up""#),
            "{html}"
        );
    }

    #[test]
    fn the_themes_tab_draws_every_theme_in_its_own_colours() {
        let html = render(|| {
            use_context_provider(|| AppState::load(&AppEnv::isolated()));
            rsx! { ThemesEditor {} }
        });
        let dark = html.find("Dark themes").expect("dark group");
        let light = html.find("Light themes").expect("light group");
        let nord = html.find(r#"aria-label="Nord""#).expect("nord");
        let paper = html.find(r#"aria-label="Paper""#).expect("paper");
        assert!(dark < nord && nord < light && light < paper, "{html}");
        assert!(html.contains("--bg:#2e3440;"), "{html}");
        assert!(html.contains("dark mode"), "{html}");
        assert!(html.contains("light mode"), "{html}");
        assert!(
            html.contains(r#"role="radio" aria-checked=true>follow the system"#),
            "{html}"
        );
        // No config directory: nothing to copy to.
        assert!(!html.contains("edit a copy"), "{html}");
        // Font names are quoted in CSS; the attribute escapes the quotes.
        assert!(
            html.contains("--font-family:&#34;JetBrains Mono&#34;, "),
            "{html}"
        );
    }

    #[test]
    fn the_general_tab_sizes_the_keyboard() {
        let html = render(|| {
            use_context_provider(|| AppState::load(&AppEnv::isolated()));
            rsx! { GeneralSettings {} }
        });
        for label in ["Keyboard size", "Window share", "Full width", "Position"] {
            assert!(html.contains(label), "{label}: {html}");
        }
        assert!(html.contains("Share of window"), "{html}");
    }

    #[test]
    fn the_general_tab_offers_auto_pair() {
        let html = render(|| {
            use_context_provider(|| AppState::load(&AppEnv::isolated()));
            rsx! { GeneralSettings {} }
        });
        assert!(
            html.contains(r#"<label for="auto-pair-brackets-and-quotes">Auto-pair brackets and quotes</label>"#),
            "{html}"
        );
    }

    #[test]
    fn the_vietnamese_tab_shows_both_methods_and_their_rules() {
        let html = render(|| {
            use_context_provider(|| AppState::load(&AppEnv::isolated()));
            rsx! { VietnameseSettings {} }
        });
        for text in [
            "Tiếng Việt",
            r#"role="switch" aria-checked=false"#,
            ">Telex</span>",
            ">VNI</span>",
            ">VIQR</span>",
            "A lone w types ư",
            "[ and ] type ơ and ư",
            "Tiếng Việt</span>",
            "hòa khỏe thủy",
            "hoà khoẻ thuỷ",
            "Xin chào các bạn",
            "Không có gì quý hơn độc lập tự do",
            "Check spelling",
            "Precomposed (NFC)",
            "Decomposed (NFD)",
            "Việt is 4 characters",
            "Việt is 6 characters",
        ] {
            assert!(html.contains(text), "{text}: {html}");
        }
        // Telex is the default: its card is the one in use.
        let telex = html.find(">Telex</span>").unwrap();
        let vni = html.find(">VNI</span>").unwrap();
        let in_use = html.find(">in use</span>").unwrap();
        assert!(telex < in_use && in_use < vni, "{html}");
    }

    #[test]
    fn the_profiles_tab_lists_the_menu() {
        let html = render(|| {
            use_context_provider(|| AppState::load(&AppEnv::isolated()));
            rsx! { ProfilesEditor {} }
        });
        for title in ["Coding", "Coding with a number row", "Terminal", "Compact"] {
            assert!(
                html.contains(&format!(">{title}</label>")),
                "{title}: {html}"
            );
        }
        assert!(html.contains("in use"), "{html}");
        assert!(html.contains("copy this profile"), "{html}");
    }
}
