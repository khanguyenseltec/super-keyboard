//! The bar above the preview: layer, the Vietnamese switch, profile picker and
//! panel buttons.

use dioxus::prelude::*;

use crate::state::{AppState, Panel};

#[component]
pub fn TopBar() -> Element {
    let mut state = use_context::<AppState>();
    let (active, profiles) = {
        let config = state.config.read();
        let active = config.active_profile().name().to_owned();
        // The menu's profiles, plus the active one even if it is hidden.
        let profiles: Vec<(String, String)> = config
            .menu()
            .into_iter()
            .filter(|(profile, shown)| *shown || profile.name() == active)
            .map(|(profile, _)| (profile.name().to_owned(), profile.title().to_owned()))
            .collect();
        (active, profiles)
    };
    let (layer, on_base) = {
        let keyboard = state.keyboard.keyboard.read();
        (keyboard.layer().name.clone(), keyboard.layer_index() == 0)
    };
    let settings_open = (state.panel)() == Panel::Settings;
    let vi = state.config.read().settings.vietnamese;
    let vi_title = if vi.enabled {
        format!(
            "Vietnamese ({}): on. Tap for plain keys.",
            vi.method.label()
        )
    } else {
        "Vietnamese: off. Tap to type Vietnamese.".to_owned()
    };

    rsx! {
        header { class: "bar",
            span { class: "title", "Super Keyboard" }
            button {
                class: "status",
                aria_label: "Layer {layer}",
                title: if on_base { "The base layer" } else { "Back to the base layer" },
                disabled: on_base,
                onclick: move |_| state.keyboard.show_base_layer(),
                "{layer}"
            }
            button {
                class: "lang-toggle",
                role: "switch",
                aria_checked: vi.enabled,
                aria_label: "Vietnamese typing",
                title: "{vi_title}",
                onclick: move |_| state.toggle_vietnamese(),
                span { class: "lang-code", if vi.enabled { "VI" } else { "EN" } }
                span { class: "lang-method", "{vi.method.label()}" }
            }
            select {
                class: "profile-picker",
                aria_label: "Profile",
                onchange: move |evt| state.select_profile(evt.value()),
                for (name, title) in profiles {
                    option {
                        key: "{name}",
                        value: "{name}",
                        selected: name == active,
                        "{title}"
                    }
                }
            }
            button {
                class: "bar-button",
                aria_pressed: settings_open,
                onclick: move |_| state.toggle_panel(Panel::Settings),
                if settings_open { "done" } else { "settings" }
            }
            button {
                class: "bar-button",
                onclick: move |_| state.keyboard.clear_preview(),
                "clear"
            }
        }
    }
}
