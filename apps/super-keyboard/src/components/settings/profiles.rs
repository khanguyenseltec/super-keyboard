//! The Profiles tab: choose, name, copy and delete profiles, and arrange the
//! profile menu (order, and which profiles it shows).

use dioxus::prelude::*;
use super_keyboard_config::{Profile, ProfileOrigin};

use super::fields::{MoveButtons, TextField};
use crate::state::AppState;

/// One profile as the list shows it.
#[derive(Debug, Clone, PartialEq)]
struct ProfileRow {
    name: String,
    title: String,
    origin: ProfileOrigin,
    layout: String,
    shown: bool,
    active: bool,
}

impl ProfileRow {
    fn new(profile: &Profile, shown: bool, active: &str) -> Self {
        Self {
            name: profile.name().to_owned(),
            title: profile.title().to_owned(),
            origin: profile.origin,
            layout: profile.spec.layout.clone(),
            shown,
            active: profile.name() == active,
        }
    }

    /// What removing the user's copy does, if there is one to remove.
    fn removal(&self) -> Option<&'static str> {
        match self.origin {
            ProfileOrigin::User => Some("delete"),
            ProfileOrigin::CustomizedPreset => Some("reset to preset"),
            ProfileOrigin::Preset | ProfileOrigin::Layout => None,
        }
    }
}

#[component]
pub fn ProfilesEditor() -> Element {
    let mut state = use_context::<AppState>();
    let (rows, title) = {
        let config = state.config.read();
        let active = config.active_profile();
        let rows: Vec<ProfileRow> = config
            .menu()
            .into_iter()
            .map(|(profile, shown)| ProfileRow::new(profile, shown, active.name()))
            .collect();
        (rows, active.title().to_owned())
    };
    let count = rows.len();

    rsx! {
        div { class: "settings-tab",
            div { class: "fields",
                TextField {
                    label: "Profile name",
                    value: title,
                    onchange: move |title: String| state.rename_profile(title),
                }
            }
            p { class: "hint",
                "The profile menu, top to bottom. Untick a profile to hide it from the menu."
            }
            ol { class: "arrange-list", aria_label: "Profiles",
                for (i, row) in rows.into_iter().enumerate() {
                    ProfileItem {
                        key: "{row.name}",
                        row,
                        first: i == 0,
                        last: i + 1 == count,
                    }
                }
            }
            div { class: "actions",
                button {
                    class: "bar-button",
                    onclick: move |_| state.new_profile(),
                    "copy this profile"
                }
            }
        }
    }
}

#[component]
fn ProfileItem(row: ProfileRow, first: bool, last: bool) -> Element {
    let mut state = use_context::<AppState>();
    let name = row.name.clone();
    let (show_name, move_name, use_name, remove_name) =
        (name.clone(), name.clone(), name.clone(), name);
    let id = format!("profile-{}", row.name);
    let removal = row.removal();

    rsx! {
        li { class: if row.active { "arrange-item here" } else { "arrange-item" },
            MoveButtons {
                name: row.title.clone(),
                first,
                last,
                onmove: move |by| state.move_in_menu(move_name.clone(), by),
            }
            input {
                id: "{id}",
                r#type: "checkbox",
                title: "Show in the menu",
                checked: row.shown,
                onchange: move |evt| state.set_in_menu(show_name.clone(), evt.checked()),
            }
            label { r#for: "{id}", class: "arrange-title", "{row.title}" }
            span { class: "tags",
                span { class: "tag", "{row.origin.label()}" }
                span { class: "tag", "{row.layout}" }
            }
            span { class: "row-actions",
                if row.active {
                    span { class: "tag here", "in use" }
                } else {
                    button {
                        class: "bar-button",
                        onclick: move |_| state.select_profile(use_name.clone()),
                        "use"
                    }
                }
                if let Some(label) = removal {
                    button {
                        class: "bar-button",
                        onclick: move |_| state.remove_profile(remove_name.clone()),
                        "{label}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super_keyboard_config::{LayoutLibrary, ProfileCatalog, ProfileSpec, profile::presets};

    #[test]
    fn only_saved_profiles_can_be_removed() {
        let mut changed = presets()[0].clone();
        changed.title = "Mine".into();
        let user = [changed, ProfileSpec::new("own", "Own", "dev")];
        let catalog = ProfileCatalog::new(presets(), &user, &LayoutLibrary::builtin());
        let removal =
            |name: &str| ProfileRow::new(catalog.get(name).unwrap(), true, "coding").removal();
        assert_eq!(removal("coding"), Some("reset to preset"));
        assert_eq!(removal("own"), Some("delete"));
        assert_eq!(removal("terminal"), None);
        assert!(ProfileRow::new(catalog.get("coding").unwrap(), true, "coding").active);
    }
}
