//! The settings panel: tabs for general settings, the sections of the active
//! profile, and the profiles and their menu.
//!
//! Changes apply at once and are saved to `settings.toml`, keeping the file's
//! comments. Styles and layouts are edited as files; the panel says where.

mod fields;
mod general;
mod profiles;
mod sections;

use dioxus::prelude::*;

use crate::state::AppState;
use general::GeneralSettings;
use profiles::ProfilesEditor;
use sections::SectionsEditor;

/// A tab of the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    General,
    Sections,
    Profiles,
}

impl Tab {
    const ALL: [Self; 3] = [Self::General, Self::Sections, Self::Profiles];

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
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
