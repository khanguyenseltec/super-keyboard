//! What has been typed so far, with the cursor and selection, plus a log of
//! recent output.

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
    let recent: Vec<&str> = sink.log().rev().take(LOG_LINES).collect();

    rsx! {
        section { class: "preview",
            pre { class: "text", aria_label: "Preview",
                "{view.before}"
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
