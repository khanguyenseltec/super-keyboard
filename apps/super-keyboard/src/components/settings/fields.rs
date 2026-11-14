//! Form controls for the settings panel.

use dioxus::prelude::*;
use super_keyboard_config::Limits;

/// A labelled slider. The value shown follows the thumb while it is dragged;
/// `onchange` fires once, when it is let go, so settings are saved once per
/// adjustment rather than on every pixel.
#[component]
pub fn RangeField(
    label: &'static str,
    unit: &'static str,
    limits: Limits,
    step: u32,
    value: u32,
    #[props(default)] disabled: bool,
    onchange: EventHandler<u32>,
) -> Element {
    let mut draft = use_signal(|| None::<u32>);
    let shown = draft().unwrap_or(value);
    let id = label.to_lowercase().replace(' ', "-");

    rsx! {
        div { class: "field",
            label { r#for: "{id}", "{label}" }
            input {
                id: "{id}",
                r#type: "range",
                min: "{limits.min}",
                max: "{limits.max}",
                step: "{step}",
                value: "{shown}",
                disabled,
                oninput: move |evt| draft.set(evt.parsed().ok()),
                onchange: move |evt| {
                    draft.set(None);
                    if let Ok(value) = evt.parsed::<u32>() {
                        onchange.call(limits.clamp(value));
                    }
                },
            }
            output { class: "value", "{shown} {unit}" }
        }
    }
}

/// A labelled drop-down of `(value, label)` options.
#[component]
pub fn SelectField(
    label: &'static str,
    value: String,
    options: Vec<(String, String)>,
    onchange: EventHandler<String>,
) -> Element {
    let id = label.to_lowercase().replace(' ', "-");
    rsx! {
        div { class: "field",
            label { r#for: "{id}", "{label}" }
            select {
                id: "{id}",
                onchange: move |evt| onchange.call(evt.value()),
                for (option_value, option_label) in options {
                    option {
                        key: "{option_value}",
                        selected: option_value == value,
                        value: "{option_value}",
                        "{option_label}"
                    }
                }
            }
        }
    }
}

/// A labelled checkbox.
#[component]
pub fn ToggleField(label: &'static str, checked: bool, onchange: EventHandler<bool>) -> Element {
    let id = label.to_lowercase().replace(' ', "-");
    rsx! {
        div { class: "field toggle",
            label { r#for: "{id}", "{label}" }
            input {
                id: "{id}",
                r#type: "checkbox",
                checked,
                onchange: move |evt| onchange.call(evt.checked()),
            }
        }
    }
}

/// "Move up" and "move down" buttons for one entry of a reorderable list.
#[component]
pub fn MoveButtons(
    /// What is being moved, for screen readers.
    name: String,
    first: bool,
    last: bool,
    onmove: EventHandler<isize>,
) -> Element {
    rsx! {
        span { class: "move",
            button {
                class: "icon-button",
                aria_label: "Move {name} up",
                title: "Move up",
                disabled: first,
                onclick: move |_| onmove.call(-1),
                "▲"
            }
            button {
                class: "icon-button",
                aria_label: "Move {name} down",
                title: "Move down",
                disabled: last,
                onclick: move |_| onmove.call(1),
                "▼"
            }
        }
    }
}

/// A labelled text box. `onchange` fires when the text is committed (Enter or
/// leaving the box), not on every keystroke.
#[component]
pub fn TextField(label: &'static str, value: String, onchange: EventHandler<String>) -> Element {
    let id = label.to_lowercase().replace(' ', "-");
    rsx! {
        div { class: "field",
            label { r#for: "{id}", "{label}" }
            input {
                id: "{id}",
                r#type: "text",
                value: "{value}",
                maxlength: 60,
                onchange: move |evt| onchange.call(evt.value()),
            }
        }
    }
}
