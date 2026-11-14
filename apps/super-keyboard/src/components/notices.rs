//! Problems with the user's config files, shown instead of failing silently.

use dioxus::prelude::*;

use crate::state::AppState;

#[component]
pub fn Notices() -> Element {
    let state = use_context::<AppState>();
    let config = state.config.read();
    if config.problems.is_empty() {
        return rsx! {};
    }
    rsx! {
        ul { class: "notices", role: "status",
            for problem in config.problems.iter() {
                li { key: "{problem}",
                    span { class: "file", "{problem.file}" }
                    " {problem.message}"
                }
            }
        }
    }
}
