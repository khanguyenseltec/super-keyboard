//! The Vietnamese tab: the on/off switch, Telex or VNI, tone placement,
//! spelling, and a box to try keys in.
//!
//! Every result shown (`aa` → `â`) is typed by the engine as the tab renders,
//! so the cheat sheets always match what the keyboard does.

use dioxus::prelude::*;
use super_keyboard_config::Settings;
use super_keyboard_core::{InputMethod, ToneStyle, VietnameseOptions, vietnamese::transliterate};

use super::fields::ToggleField;
use crate::state::AppState;

/// A rule on a method's cheat sheet: the keys, and what they are called.
type Rule = (&'static str, &'static str);

const TELEX_TONES: [Rule; 6] = [
    ("as", "sắc"),
    ("af", "huyền"),
    ("ar", "hỏi"),
    ("ax", "ngã"),
    ("aj", "nặng"),
    ("asz", "xoá dấu"),
];
const TELEX_LETTERS: [Rule; 7] = [
    ("aa", ""),
    ("aw", ""),
    ("ee", ""),
    ("oo", ""),
    ("ow", ""),
    ("uw", "or w"),
    ("dd", ""),
];
const VNI_TONES: [Rule; 6] = [
    ("a1", "sắc"),
    ("a2", "huyền"),
    ("a3", "hỏi"),
    ("a4", "ngã"),
    ("a5", "nặng"),
    ("a10", "xoá dấu"),
];
const VNI_LETTERS: [Rule; 7] = [
    ("a6", ""),
    ("a8", ""),
    ("e6", ""),
    ("o6", ""),
    ("o7", ""),
    ("u7", ""),
    ("d9", ""),
];

/// What each method is, in a line, and the sentence its card types.
fn about(method: InputMethod) -> (&'static str, &'static str, [Rule; 6], [Rule; 7]) {
    match method {
        InputMethod::Telex => (
            "Letters add the marks. The most popular way to type Vietnamese.",
            "Tieesng Vieejt",
            TELEX_TONES,
            TELEX_LETTERS,
        ),
        InputMethod::Vni => (
            "Digits add the marks, after the letter they belong to.",
            "Tie61ng Vie65t",
            VNI_TONES,
            VNI_LETTERS,
        ),
    }
}

/// Phrases to try, as keys for `method`.
fn examples(method: InputMethod) -> [&'static str; 4] {
    match method {
        InputMethod::Telex => [
            "Xin chaof cacs banj",
            "Tooi yeeu Vieejt Nam",
            "dduwowngf phoos",
            "Khoong cos gif quys hown ddoocj laapj tuwj do",
        ],
        InputMethod::Vni => [
            "Xin cha2o ca1c ba5n",
            "To6i ye6u Vie65t Nam",
            "d9u7o7ng2 pho61",
            "Kho6ng co1 gi2 quy1 ho7n d9o65c la65p tu75 do",
        ],
    }
}

#[component]
pub fn VietnameseSettings() -> Element {
    let mut state = use_context::<AppState>();
    let vi = state.config.read().settings.vietnamese;
    // What the user typed in the try-it box; `None` shows the method's first
    // example, so switching method shows keys that suit it.
    let mut keys = use_signal(|| None::<String>);
    let shown_keys = keys().unwrap_or_else(|| examples(vi.method)[0].to_owned());
    let typed = transliterate(&shown_keys, vi);
    let status = if vi.enabled {
        format!("On · {}", vi.method.label())
    } else {
        "Off · keys type what they show".to_owned()
    };

    rsx! {
        div { class: "settings-tab vn",
            section { class: "vn-hero", "data-on": vi.enabled,
                div { class: "vn-hero-text",
                    h3 { class: "vn-title", "Tiếng Việt" }
                    p { class: "vn-subtitle", "Type Vietnamese with Telex or VNI, right on the keys." }
                    p { class: "vn-status", span { class: "vn-dot" } "{status}" }
                }
                button {
                    class: "switch",
                    role: "switch",
                    aria_checked: vi.enabled,
                    aria_label: "Vietnamese typing",
                    onclick: move |_| state.toggle_vietnamese(),
                    span { class: "switch-thumb" }
                }
            }

            h3 { class: "group-title", "Input method · Kiểu gõ" }
            div { class: "vn-methods", role: "radiogroup", aria_label: "Input method",
                for method in InputMethod::ALL {
                    MethodCard {
                        key: "{method.name()}",
                        method,
                        options: vi,
                        onselect: move |method| {
                            keys.set(None);
                            state.update_settings(|s| s.vietnamese.method = method);
                        },
                    }
                }
            }

            h3 { class: "group-title", "Tone placement · Vị trí dấu" }
            div { class: "segmented", role: "radiogroup", aria_label: "Tone placement",
                for style in ToneStyle::ALL {
                    button {
                        key: "{style.name()}",
                        class: "segment",
                        role: "radio",
                        aria_checked: vi.tone_style == style,
                        onclick: move |_| state.update_settings(|s| s.vietnamese.tone_style = style),
                        span { class: "segment-label", "{style.label()}" }
                        span { class: "segment-sample", "{tone_sample(vi, style)}" }
                    }
                }
            }
            p { class: "hint",
                "Where the tone goes in " strong { "oa" } ", " strong { "oe" } " and " strong { "uy" }
                " at the end of a word. Both are correct; classic is the more common."
            }

            h3 { class: "group-title", "Options" }
            div { class: "fields",
                ToggleField {
                    label: "Check spelling",
                    checked: vi.check_spelling,
                    onchange: move |on: bool| state.update_settings(|s| s.vietnamese.check_spelling = on),
                }
            }
            p { class: "hint",
                "Only syllables that can be Vietnamese change, so " code { "for" } ", "
                code { "class" } " and most code type as written. Typing a mark's key twice takes it off: "
                code { "{undo_sample(vi.method)}" } "."
            }

            h3 { class: "group-title", "Try it · Gõ thử" }
            div { class: "vn-try",
                input {
                    class: "vn-try-input",
                    r#type: "text",
                    aria_label: "Keys to type",
                    placeholder: "Type keys here",
                    autocomplete: "off",
                    spellcheck: "false",
                    value: "{shown_keys}",
                    oninput: move |evt| keys.set(Some(evt.value())),
                }
                output { class: "vn-try-output", aria_live: "polite", "{typed}" }
            }
            div { class: "vn-chips",
                for example in examples(vi.method) {
                    button {
                        key: "{example}",
                        class: "chip",
                        onclick: move |_| keys.set(Some(example.to_owned())),
                        "{transliterate(example, vi)}"
                    }
                }
            }

            if let Some(installed) = SystemEngine::detect() {
                h3 { class: "group-title", "Physical keyboard · Bàn phím thật" }
                SystemEngineCard { installed }
            }

            div { class: "actions",
                button {
                    class: "bar-button",
                    onclick: move |_| {
                        keys.set(None);
                        state.update_settings(reset_vietnamese);
                    },
                    "reset to defaults"
                }
            }
            p { class: "hint",
                "The on-screen keys type into the preview for now; typing into other apps with them comes with the system keyboards (Android in v0.2, the Ubuntu desktop in v0.3)."
            }
        }
    }
}

/// The IBus engine that types Vietnamese with the physical keyboard in every
/// application (`apps/super-keyboard-ibus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemEngine {
    Installed,
    Missing,
}

impl SystemEngine {
    /// Where IBus finds the engine once it is installed.
    const COMPONENT: &str = "/usr/share/ibus/component/super-keyboard.xml";

    /// On Linux desktops, whether the engine is installed; elsewhere `None`.
    fn detect() -> Option<Self> {
        if cfg!(all(target_os = "linux", not(target_arch = "wasm32"))) {
            Some(if std::path::Path::new(Self::COMPONENT).exists() {
                Self::Installed
            } else {
                Self::Missing
            })
        } else {
            None
        }
    }
}

#[component]
fn SystemEngineCard(installed: SystemEngine) -> Element {
    let on = installed == SystemEngine::Installed;
    rsx! {
        section { class: "vn-system", "data-on": on,
            div { class: "vn-system-head",
                span { class: "vn-system-badge", if on { "Installed" } else { "Not installed" } }
                strong { "Type Vietnamese with your physical keyboard in every app" }
            }
            if on {
                p {
                    "Pick " strong { "Vietnamese (Super Keyboard)" }
                    " in your input sources and switch with "
                    kbd { "Super" } " + " kbd { "Space" }
                    ". It uses the settings on this tab, and the two stay in sync: the VI button here and the VI / EN menu in the top bar of the desktop change both."
                }
            } else {
                p {
                    "Install the Vietnamese input method for IBus: it comes with the "
                    code { ".deb" } ", or from the source run "
                    code { "apps/super-keyboard-ibus/install.sh" }
                    ". It types with these same settings in every application, and stays in sync with this app."
                }
            }
        }
    }
}

/// A method to choose, with its cheat sheet.
#[component]
fn MethodCard(
    method: InputMethod,
    options: VietnameseOptions,
    onselect: EventHandler<InputMethod>,
) -> Element {
    let selected = options.method == method;
    let options = VietnameseOptions { method, ..options };
    let (description, sample, tones, letters) = about(method);
    let rule = |(keys, name): Rule| (keys, transliterate(keys, options), name);
    let tones: Vec<_> = tones.into_iter().map(rule).collect();
    let letters: Vec<_> = letters.into_iter().map(rule).collect();

    rsx! {
        button {
            class: "vn-method",
            role: "radio",
            aria_checked: selected,
            onclick: move |_| onselect.call(method),
            div { class: "vn-method-head",
                span { class: "vn-method-name", "{method.label()}" }
                if selected {
                    span { class: "tag here", "in use" }
                }
            }
            p { class: "vn-method-desc", "{description}" }
            p { class: "vn-example",
                kbd { "{sample}" }
                span { class: "vn-arrow", aria_hidden: true, "→" }
                span { class: "vn-example-out", "{transliterate(sample, options)}" }
            }
            div { class: "vn-rules", aria_label: "Tones",
                for (keys, out, name) in tones {
                    RuleCell { key: "{keys}", keys, out, name }
                }
            }
            div { class: "vn-rules", aria_label: "Letters",
                for (keys, out, name) in letters {
                    RuleCell { key: "{keys}", keys, out, name }
                }
            }
        }
    }
}

#[component]
fn RuleCell(keys: &'static str, out: String, name: &'static str) -> Element {
    rsx! {
        span { class: "vn-rule",
            kbd { "{keys}" }
            span { class: "vn-rule-out", "{out}" }
            if !name.is_empty() {
                span { class: "vn-rule-name", "{name}" }
            }
        }
    }
}

/// `hòa khỏe thủy` or `hoà khoẻ thuỷ`.
fn tone_sample(options: VietnameseOptions, style: ToneStyle) -> String {
    let options = VietnameseOptions {
        method: InputMethod::Telex,
        tone_style: style,
        ..options
    };
    transliterate("hoaf khoer thuyr", options)
}

/// `ass → as` or `a11 → a1`.
fn undo_sample(method: InputMethod) -> &'static str {
    match method {
        InputMethod::Telex => "ass → as",
        InputMethod::Vni => "a11 → a1",
    }
}

/// Resets what this tab controls, keeping Vietnamese on or off as it is.
fn reset_vietnamese(settings: &mut Settings) {
    settings.vietnamese = VietnameseOptions {
        enabled: settings.vietnamese.enabled,
        ..VietnameseOptions::default()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheat_sheets_show_what_the_keys_make() {
        let telex = VietnameseOptions::default();
        let made: Vec<String> = TELEX_LETTERS
            .iter()
            .map(|(keys, _)| transliterate(keys, telex))
            .collect();
        assert_eq!(made, ["â", "ă", "ê", "ô", "ơ", "ư", "đ"]);
        let vni = VietnameseOptions {
            method: InputMethod::Vni,
            ..telex
        };
        let made: Vec<String> = VNI_LETTERS
            .iter()
            .map(|(keys, _)| transliterate(keys, vni))
            .collect();
        assert_eq!(made, ["â", "ă", "ê", "ô", "ơ", "ư", "đ"]);
        for tones in [(TELEX_TONES, telex), (VNI_TONES, vni)] {
            let made: Vec<String> = tones
                .0
                .iter()
                .map(|(k, _)| transliterate(k, tones.1))
                .collect();
            assert_eq!(made, ["á", "à", "ả", "ã", "ạ", "a"]);
        }
        assert_eq!(tone_sample(telex, ToneStyle::Classic), "hòa khỏe thủy");
        assert_eq!(tone_sample(telex, ToneStyle::Modern), "hoà khoẻ thuỷ");
    }

    #[test]
    fn examples_type_the_same_phrases_in_both_methods() {
        let telex = VietnameseOptions::default();
        let vni = VietnameseOptions {
            method: InputMethod::Vni,
            ..telex
        };
        for (t, v) in examples(InputMethod::Telex)
            .iter()
            .zip(examples(InputMethod::Vni))
        {
            assert_eq!(transliterate(t, telex), transliterate(v, vni), "{t} / {v}");
        }
        assert_eq!(
            transliterate(about(InputMethod::Telex).1, telex),
            transliterate(about(InputMethod::Vni).1, vni)
        );
    }

    #[test]
    fn the_system_engine_is_only_offered_on_linux_desktops() {
        let detected = SystemEngine::detect();
        assert_eq!(
            detected.is_some(),
            cfg!(all(target_os = "linux", not(target_arch = "wasm32")))
        );
    }

    #[test]
    fn reset_keeps_the_switch() {
        let mut settings = Settings {
            vietnamese: VietnameseOptions {
                enabled: true,
                method: InputMethod::Vni,
                tone_style: ToneStyle::Modern,
                check_spelling: false,
            },
            ..Settings::default()
        };
        reset_vietnamese(&mut settings);
        assert_eq!(
            settings.vietnamese,
            VietnameseOptions {
                enabled: true,
                ..VietnameseOptions::default()
            }
        );
    }
}
