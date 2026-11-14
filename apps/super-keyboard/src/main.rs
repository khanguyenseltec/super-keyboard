//! Super Keyboard app: the Dioxus front end shared by Ubuntu desktop and Android.
//!
//! Today the app types into its own preview panel through
//! [`PreviewSink`]. Platform sinks that type into other applications are tracked in
//! `docs/ROADMAP.md`.

use dioxus::prelude::*;
use super_keyboard_core::{Action, Key, Keyboard, Latch, Layout, OutputSink, PreviewSink};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let keyboard = use_signal(|| Keyboard::new(Layout::builtin()));
    let sink = use_signal(PreviewSink::default);

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        div { class: "app",
            Preview { keyboard, sink }
            KeyboardView { keyboard, sink }
        }
    }
}

/// What has been typed so far, plus a log of recent output.
#[component]
fn Preview(keyboard: Signal<Keyboard>, mut sink: Signal<PreviewSink>) -> Element {
    let kb = keyboard.read();
    let status = format!("{} · {}", kb.layout().name, kb.layer().name);
    let preview = sink.read();
    let text = preview.text().to_owned();
    let recent: Vec<String> = preview.log().iter().rev().take(8).cloned().collect();

    rsx! {
        header { class: "bar",
            span { class: "title", "Super Keyboard" }
            span { class: "status", "{status}" }
            button { class: "clear", onclick: move |_| sink.write().clear(), "clear" }
        }
        section { class: "preview",
            pre { class: "text",
                "{text}"
                span { class: "caret" }
            }
            ol { class: "log",
                for (i, entry) in recent.into_iter().enumerate() {
                    li { key: "{i}", "{entry}" }
                }
            }
        }
    }
}

/// The keys of the current layer.
#[component]
fn KeyboardView(mut keyboard: Signal<Keyboard>, mut sink: Signal<PreviewSink>) -> Element {
    let kb = keyboard.read();
    let rows: Vec<Vec<(String, String, f32)>> = kb
        .layer()
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|key| {
                    (
                        kb.display_label(key).into_owned(),
                        key_class(&kb, key),
                        key.width,
                    )
                })
                .collect()
        })
        .collect();

    rsx! {
        div { class: "keyboard",
            for (r, row) in rows.into_iter().enumerate() {
                div { key: "{r}", class: "row",
                    for (c, (label, class, width)) in row.into_iter().enumerate() {
                        button {
                            key: "{c}",
                            class: "{class}",
                            style: "flex-grow: {width}",
                            // Pointer-down rather than click: keys fire on touch, not release.
                            onpointerdown: move |evt| {
                                evt.prevent_default();
                                let output = keyboard.write().tap(r, c);
                                if let Some(output) = output {
                                    sink.write().emit(output);
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

fn key_class(kb: &Keyboard, key: &Key) -> String {
    match &key.action {
        Action::Modifier(modifier) => match kb.latch(*modifier) {
            Latch::Off => "key mod".into(),
            Latch::OneShot => "key mod oneshot".into(),
            Latch::Locked => "key mod locked".into(),
        },
        Action::Layer(name) if *name == kb.layer().name => "key layer active".into(),
        Action::Layer(_) => "key layer".into(),
        Action::Key(_) => "key named".into(),
        Action::Text(text) if text.chars().count() > 1 => "key op".into(),
        Action::Text(_) => "key".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders the app without a webview, so the UI is checked on any CI host.
    #[test]
    fn renders_the_base_layer() {
        let mut dom = VirtualDom::new(App);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Super Keyboard"), "{html}");
        assert!(html.contains(">q</button>"), "{html}");
        assert!(html.contains("dev · base"), "{html}");
    }
}
