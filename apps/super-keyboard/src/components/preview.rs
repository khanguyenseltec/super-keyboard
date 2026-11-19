//! What has been typed so far, with the cursor and selection, plus a log of
//! recent output. The Vietnamese syllable being typed is underlined.

use dioxus::prelude::*;

use crate::state::AppState;

/// How many log entries are shown.
const LOG_LINES: usize = 8;

#[component]
pub fn Preview() -> Element {
    let state = use_context::<AppState>();
    let show_log = state.config.read().settings.preview.show_log;
    let sink = state.keyboard.sink.read();
    let view = sink.buffer().view();
    let composing = state.keyboard.vietnamese.read().composing();
    // The syllable sits just before the cursor while it is being typed.
    let (before, composing) = match view.before.strip_suffix(composing.as_str()) {
        Some(rest) if !composing.is_empty() && view.selected.is_empty() => (rest, composing),
        _ => (view.before, String::new()),
    };
    let recent: Vec<&str> = sink.log().rev().take(LOG_LINES).collect();

    rsx! {
        section { class: "preview",
            pre { class: "text", aria_label: "Preview",
                "{before}"
                if !composing.is_empty() {
                    span { class: "composing", "{composing}" }
                }
                if view.cursor_at_start {
                    Caret {}
                }
                if !view.selected.is_empty() {
                    mark { class: "selection", "{view.selected}" }
                }
                if !view.cursor_at_start {
                    Caret {}
                }
                "{view.after}"
            }
            if show_log {
                ol { class: "log", aria_label: "Recent output",
                    for (i, entry) in recent.into_iter().enumerate() {
                        li { key: "{i}", "{entry}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Caret() -> Element {
    rsx! {
        span { class: "caret" }
    }
}
