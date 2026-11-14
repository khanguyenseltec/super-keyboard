//! The Themes tab: every theme as a card drawn in its own colours and fonts.
//! Follow the system's light and dark mode with one theme for each, or keep
//! one theme; copy a built-in theme to a file to change it.

use dioxus::prelude::*;
use super_keyboard_config::{ActiveTheme, Appearance, ThemeEntry};

use crate::{state::AppState, style::theme_preview_style};

/// Whether theme files can be edited here: on the desktop, where the app
/// reloads them as they are saved.
const CAN_EDIT_FILES: bool = cfg!(not(any(target_os = "android", target_arch = "wasm32")));

/// One theme as the list shows it.
#[derive(Debug, Clone, PartialEq)]
struct ThemeCard {
    id: String,
    title: String,
    description: String,
    appearance: Appearance,
    /// The theme's variables, as an inline style.
    style: String,
    /// How the theme is in use, if it is.
    in_use: Option<&'static str>,
    builtin: bool,
}

impl ThemeCard {
    fn new(entry: &ThemeEntry, active: &ActiveTheme) -> Self {
        Self {
            id: entry.id.clone(),
            title: entry.theme.title.clone(),
            description: entry.theme.description.clone(),
            appearance: entry.theme.appearance,
            style: theme_preview_style(&entry.theme),
            in_use: in_use(active, &entry.id),
            builtin: entry.is_builtin(),
        }
    }
}

fn in_use(active: &ActiveTheme, id: &str) -> Option<&'static str> {
    match active {
        ActiveTheme::Fixed(entry) => (entry.id == id).then_some("in use"),
        ActiveTheme::System { light, dark } => match (light.id == id, dark.id == id) {
            (true, true) => Some("light and dark mode"),
            (true, false) => Some("light mode"),
            (false, true) => Some("dark mode"),
            (false, false) => None,
        },
    }
}

#[component]
pub fn ThemesEditor() -> Element {
    let mut state = use_context::<AppState>();
    let (following, cards, themes_dir, can_copy) = {
        let config = state.config.read();
        let active = config.active_theme();
        let cards: Vec<ThemeCard> = config
            .themes
            .entries()
            .iter()
            .map(|entry| ThemeCard::new(entry, &active))
            .collect();
        let dir = config.dir.as_ref();
        (
            matches!(active, ActiveTheme::System { .. }),
            cards,
            dir.map(|d| d.themes_dir().display().to_string()),
            CAN_EDIT_FILES && dir.is_some(),
        )
    };

    rsx! {
        div { class: "settings-tab",
            div { class: "theme-mode", role: "radiogroup", aria_label: "Theme",
                button {
                    class: "bar-button",
                    role: "radio",
                    aria_checked: following,
                    onclick: move |_| state.follow_system(true),
                    "follow the system"
                }
                button {
                    class: "bar-button",
                    role: "radio",
                    aria_checked: !following,
                    onclick: move |_| state.follow_system(false),
                    "one theme"
                }
            }
            p { class: "hint",
                if following {
                    "Pick a dark theme for when the system is dark, and a light theme for when it is light."
                } else {
                    "Pick the theme to use, whether the system is light or dark."
                }
            }
            for appearance in [Appearance::Dark, Appearance::Light] {
                section { key: "{appearance}", class: "settings-group",
                    h3 { class: "group-title", "{appearance.label()} themes" }
                    ul { class: "theme-grid", aria_label: "{appearance.label()} themes",
                        for card in cards.iter().filter(|c| c.appearance == appearance).cloned() {
                            ThemeItem { key: "{card.id}", card, can_copy }
                        }
                    }
                }
            }
            if let Some(dir) = themes_dir {
                p { class: "hint",
                    "Add your own themes as " code { "*.json" } " files in " code { "{dir}" }
                    ". A file named like a built-in theme replaces it."
                }
            }
        }
    }
}

#[component]
fn ThemeItem(card: ThemeCard, can_copy: bool) -> Element {
    let mut state = use_context::<AppState>();
    let mut notice = use_signal(|| None::<String>);
    let (use_id, copy_id) = (card.id.clone(), card.id.clone());
    let copyable = can_copy && card.builtin && card.in_use.is_some();

    rsx! {
        li { class: if card.in_use.is_some() { "theme-item here" } else { "theme-item" },
            button {
                class: "theme-card",
                style: "{card.style}",
                aria_pressed: card.in_use.is_some(),
                aria_label: "{card.title}",
                title: "{card.description}",
                onclick: move |_| state.choose_theme(use_id.clone()),
                span { class: "theme-title", "{card.title}" }
                span { class: "theme-sample", aria_hidden: true,
                    span { class: "sample-key special", "esc" }
                    span { class: "sample-key", "a" }
                    span { class: "sample-key", "{{" }
                    span { class: "sample-key locked", "ctrl" }
                }
                span { class: "theme-code", aria_hidden: true,
                    "fn main() "
                    span { class: "muted", "// ok" }
                }
            }
            span { class: "tags",
                if let Some(tag) = card.in_use {
                    span { class: "tag here", "{tag}" }
                }
                if !card.builtin {
                    span { class: "tag", "yours" }
                }
                if copyable {
                    button {
                        class: "bar-button",
                        title: "Save a copy in your themes folder to edit; it replaces the built-in theme",
                        onclick: move |_| {
                            let saved = state.config.write().create_theme_file(&copy_id);
                            notice.set(Some(match saved {
                                Ok(path) => format!("Saved to {}. Edit it: the theme changes as you save.", path.display()),
                                Err(err) => err.to_string(),
                            }));
                        },
                        "edit a copy"
                    }
                }
            }
            if let Some(message) = notice() {
                p { class: "hint", role: "status", "{message}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super_keyboard_config::{Settings, UserConfig};

    fn cards(settings: &str) -> Vec<ThemeCard> {
        let config = UserConfig {
            settings: Settings::from_toml(settings).unwrap(),
            ..UserConfig::default()
        };
        let active = config.active_theme();
        config
            .themes
            .entries()
            .iter()
            .map(|entry| ThemeCard::new(entry, &active))
            .collect()
    }

    fn tag(cards: &[ThemeCard], id: &str) -> Option<&'static str> {
        cards.iter().find(|c| c.id == id).unwrap().in_use
    }

    #[test]
    fn cards_say_how_their_theme_is_used() {
        let system = cards("dark-theme = \"nord\"");
        assert_eq!(tag(&system, "nord"), Some("dark mode"));
        assert_eq!(tag(&system, "light"), Some("light mode"));
        assert_eq!(tag(&system, "dark"), None);

        let fixed = cards("theme = \"paper\"");
        assert_eq!(tag(&fixed, "paper"), Some("in use"));
        assert_eq!(tag(&fixed, "light"), None);
        assert_eq!(fixed.iter().filter(|c| c.in_use.is_some()).count(), 1);

        let same = cards("light-theme = \"dark\"");
        assert_eq!(tag(&same, "dark"), Some("light and dark mode"));
    }

    #[test]
    fn cards_carry_their_theme_as_an_inline_style() {
        let cards = cards("");
        let paper = cards.iter().find(|c| c.id == "paper").unwrap();
        assert!(paper.style.contains("--bg:#f4ecd8;"), "{}", paper.style);
        assert_eq!(paper.appearance, Appearance::Light);
        assert!(paper.builtin);
        assert!(cards.len() >= 24);
    }
}
