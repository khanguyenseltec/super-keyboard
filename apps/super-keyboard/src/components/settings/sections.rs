//! The Sections tab: switch the active profile's sections on and off, and move
//! them up and down. One order applies to every layer.

use dioxus::prelude::*;
use super_keyboard_core::{Arrangement, Layout};

use super::fields::{MoveButtons, SelectField};
use crate::state::AppState;

/// One section as the list shows it.
#[derive(Debug, Clone, PartialEq)]
struct SectionRow {
    name: String,
    title: String,
    on: bool,
    required: bool,
    /// The layers that show it.
    layers: Vec<String>,
}

/// The sections of `layout` in the order `arrangement` shows them.
fn section_rows(layout: &Layout, arrangement: &Arrangement) -> Vec<SectionRow> {
    let names = layout.section_names();
    arrangement
        .arrange(&names)
        .into_iter()
        .map(|index| {
            let section = &layout.sections[index];
            SectionRow {
                name: section.name.clone(),
                title: section.title.clone(),
                on: section.is_on(arrangement),
                required: section.required,
                layers: layout.layers_with(index).map(str::to_owned).collect(),
            }
        })
        .collect()
}

#[component]
pub fn SectionsEditor() -> Element {
    let mut state = use_context::<AppState>();
    let layer = state.keyboard.keyboard.read().layer().name.clone();
    let (title, origin, layout_id, layouts, rows, changed) = {
        let config = state.config.read();
        let profile = config.active_profile();
        let entry = config.active_layout();
        let layouts: Vec<(String, String)> = config
            .library
            .entries()
            .iter()
            .map(|e| (e.id.to_string(), e.id.to_string()))
            .collect();
        (
            profile.title().to_owned(),
            profile.origin.label(),
            entry.id.to_string(),
            layouts,
            section_rows(&entry.layout, &profile.spec.sections),
            !profile.spec.sections.is_default(),
        )
    };
    let count = rows.len();

    rsx! {
        div { class: "settings-tab",
            p { class: "hint",
                "Sections of "
                strong { "{title}" }
                " ({origin}), top to bottom. The order is the same on every layer; "
                "layers marked like this one are showing."
            }
            div { class: "fields",
                SelectField {
                    label: "Layout",
                    value: layout_id,
                    options: layouts,
                    onchange: move |id: String| state.set_profile_layout(id),
                }
            }
            ol { class: "arrange-list", aria_label: "Sections",
                for (i, row) in rows.into_iter().enumerate() {
                    SectionItem {
                        key: "{row.name}",
                        row,
                        first: i == 0,
                        last: i + 1 == count,
                        layer: layer.clone(),
                    }
                }
            }
            div { class: "actions",
                button {
                    class: "bar-button",
                    disabled: !changed,
                    onclick: move |_| state.reset_sections(),
                    "default sections"
                }
            }
        }
    }
}

#[component]
fn SectionItem(row: SectionRow, first: bool, last: bool, layer: String) -> Element {
    let mut state = use_context::<AppState>();
    let (toggle_name, move_name) = (row.name.clone(), row.name.clone());
    let showing = row.layers.contains(&layer);
    let id = format!("section-{}", row.name);

    rsx! {
        li { class: if row.on { "arrange-item" } else { "arrange-item off" },
            MoveButtons {
                name: row.title.clone(),
                first,
                last,
                onmove: move |by| state.move_section(move_name.clone(), by),
            }
            input {
                id: "{id}",
                r#type: "checkbox",
                checked: row.on,
                disabled: row.required,
                onchange: move |evt| state.set_section(toggle_name.clone(), evt.checked()),
            }
            label { r#for: "{id}", class: "arrange-title", "{row.title}" }
            span { class: "tags",
                if row.required {
                    span { class: "tag", "always on" }
                }
                for name in row.layers.iter() {
                    span {
                        key: "{name}",
                        class: if showing && *name == layer { "tag here" } else { "tag" },
                        "{name}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_follow_the_arrangement() {
        let layout = Layout::builtin();
        let mut arrangement = Arrangement::default();
        arrangement.set_enabled("number-row", true, false);
        arrangement.move_item(&layout.section_names(), "bottom", -100);
        let rows = section_rows(&layout, &arrangement);
        assert_eq!(rows[0].name, "bottom");
        assert!(rows[0].required && rows[0].on);
        assert_eq!(rows[0].layers, ["base", "sym", "nav"]);
        let number_row = rows.iter().find(|r| r.name == "number-row").unwrap();
        assert!(number_row.on && !number_row.required);
        assert_eq!(number_row.title, "Number row");
        assert!(!rows.iter().find(|r| r.name == "terminal").unwrap().on);
    }
}
