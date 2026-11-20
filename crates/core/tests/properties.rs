//! Property tests for the engine: invariants that must hold for any sequence of
//! taps, any gesture timing and any layout the parser accepts.

use std::time::Duration;

use proptest::prelude::*;
use super_keyboard_core::{
    Action, Arrangement, AutoPair, Composer, Gestures, InputMethod, KeyId, Keyboard, Keymap, Latch,
    Layout, Modifier, Modifiers, Motion, NamedKey, Output, OutputSink, PressKind, PreviewSink,
    TextBuffer, ToneStyle, Trigger, UnicodeForm, VietnameseOptions,
};

/// Every key id of the built-in layout.
fn builtin_ids() -> Vec<KeyId> {
    Keymap::builtin().keys().map(|(id, _)| id).collect()
}

fn builtin_key() -> impl Strategy<Value = KeyId> {
    proptest::sample::select(builtin_ids())
}

fn is_modifier_or_layer(action: &Action) -> bool {
    matches!(action, Action::Modifier(_) | Action::Layer(_))
}

proptest! {
    /// One-shot modifiers are released by any tap, hold or repeat that produces
    /// output.
    #[test]
    fn one_shots_release_after_output(
        steps in prop::collection::vec((builtin_key(), 0..3u8), 1..60),
    ) {
        let mut kb = Keyboard::new(Keymap::builtin());
        for (id, how) in steps {
            let trigger = match how {
                0 => Trigger::Tap(id),
                1 => Trigger::Hold(id),
                _ => Trigger::Repeat(id),
            };
            if kb.trigger(trigger).is_some() {
                for modifier in Modifier::ALL {
                    prop_assert_ne!(kb.latch(modifier), Latch::OneShot, "{:?}", trigger);
                }
            }
        }
    }

    /// A locked modifier stays locked until its own key is tapped.
    #[test]
    fn locked_modifiers_stay_locked(
        modifier in proptest::sample::select(Modifier::ALL.to_vec()),
        taps in prop::collection::vec(builtin_key(), 0..60),
    ) {
        let mut kb = Keyboard::new(Keymap::builtin());
        kb.apply(&Action::Modifier(modifier));
        kb.apply(&Action::Modifier(modifier));
        for id in taps {
            if kb.key(id).map(|k| &k.tap) == Some(&Action::Modifier(modifier)) {
                continue;
            }
            kb.tap(id);
            prop_assert_eq!(kb.latch(modifier), Latch::Locked);
        }
    }

    /// Tapping the same layer key twice from any other layer lands on the base
    /// layer; from that layer itself it comes back to it.
    #[test]
    fn layer_keys_toggle_through_base(
        before in prop::collection::vec(builtin_key(), 0..20),
        target in 0..3usize,
    ) {
        let mut kb = Keyboard::new(Keymap::builtin());
        for id in before {
            kb.tap(id);
        }
        let start = kb.layer_index();
        let name = kb.keymap().layers[target].name.clone();
        kb.apply(&Action::Layer(name.clone()));
        kb.apply(&Action::Layer(name));
        let expected = if start == target { target } else { 0 };
        prop_assert_eq!(kb.layer_index(), expected);
    }

    /// Tapping or holding any position, valid or not, never panics.
    #[test]
    fn any_position_is_safe(layer in 0..10usize, row in 0..20usize, col in 0..30usize) {
        let mut kb = Keyboard::new(Keymap::builtin());
        let id = KeyId { layer, row, col };
        let exists = kb.key(id).is_some();
        let tapped = kb.tap(id);
        prop_assert!(exists || tapped.is_none());
        kb.hold(id);
        kb.trigger(Trigger::Repeat(id));
    }

    /// Any layout the parser accepts can be pressed at every key, with every
    /// gesture, without panicking.
    #[test]
    fn accepted_layouts_work_at_every_key(source in layout_source()) {
        let Ok(layout) = Layout::from_toml(&source) else {
            return Ok(());
        };
        let keymap = Keymap::from(&layout);
        let mut kb = Keyboard::new(keymap.clone());
        let mut gestures = Gestures::default();
        let mut now = Duration::ZERO;
        for (id, key) in keymap.keys() {
            let kind = PressKind::of(key);
            let mut triggers = gestures.press(1, id, kind, 0.0, now);
            triggers.extend(gestures.moved(1, 40.0));
            now += Duration::from_millis(500);
            triggers.extend(gestures.tick(now));
            triggers.extend(gestures.release(1, now));
            for trigger in triggers {
                kb.trigger(trigger);
            }
            kb.display_label(key);
            kb.hold_hint(key);
        }
        prop_assert_eq!(gestures.next_deadline(), None);
    }

    /// Whatever the pointers do, every press ends when its pointer lifts, and a
    /// plain key fires exactly one tap per press.
    #[test]
    fn gestures_settle_when_every_pointer_lifts(
        events in prop::collection::vec(gesture_event(), 0..80),
    ) {
        let mut gestures = Gestures::default();
        let mut now = Duration::ZERO;
        let mut presses = 0;
        let mut taps = 0;
        let mut ends = 0;
        let mut swipe_presses = 0;
        for event in events {
            now += Duration::from_millis(event.after_ms);
            let out = match event.kind {
                EventKind::Press { kind, col, x } => {
                    presses += 1;
                    swipe_presses += usize::from(kind.swipe);
                    gestures.press(event.pointer, key(col), kind, x, now)
                }
                EventKind::Move(x) => gestures.moved(event.pointer, x),
                EventKind::Release => gestures.release(event.pointer, now),
                EventKind::Cancel => gestures.cancel(event.pointer),
                EventKind::Tick => gestures.tick(now),
            };
            taps += out.iter().filter(|t| matches!(t, Trigger::Tap(_) | Trigger::Hold(_))).count();
            ends += out.iter().filter(|t| matches!(t, Trigger::SwipeEnd(_))).count();
        }
        for pointer in 0..4 {
            let out = gestures.release(pointer, now);
            taps += out.iter().filter(|t| matches!(t, Trigger::Tap(_) | Trigger::Hold(_))).count();
            ends += out.iter().filter(|t| matches!(t, Trigger::SwipeEnd(_))).count();
        }
        prop_assert_eq!(gestures.pressed_keys().count(), 0);
        prop_assert_eq!(gestures.next_deadline(), None);
        prop_assert!(taps <= presses);
        prop_assert!(ends <= swipe_presses);
    }

    /// Any arrangement is a permutation of the sections, keeps required sections
    /// on, and shows sections in the same relative order on every layer.
    #[test]
    fn arrangements_keep_required_sections_and_one_order(arrangement in arrangement()) {
        let layout = Layout::builtin();
        let names = layout.section_names();
        let mut order = arrangement.arrange(&names);
        let position = |name: &str| order.iter().position(|&i| names[i] == name);
        let letters = position("letters");
        let bottom = position("bottom");
        order.sort_unstable();
        prop_assert_eq!(order, (0..names.len()).collect::<Vec<_>>());

        let keymap = layout.keymap(&arrangement);
        for (layer, def) in keymap.layers.iter().zip(&layout.layers) {
            prop_assert!(!layer.rows.is_empty(), "layer {} is empty", layer.name);
            let required: usize = def
                .sections
                .iter()
                .map(|&i| &layout.sections[i])
                .filter(|s| s.required)
                .map(|s| s.rows.len())
                .sum();
            prop_assert!(layer.rows.len() >= required);
        }
        // The base layer shows letters and the bottom row in the arranged order.
        let base = &keymap.layers[0];
        let row_of = |label: &str| base.rows.iter().position(|row| row.iter().any(|k| k.label == label));
        let (q, ctrl) = (row_of("q").unwrap(), row_of("ctrl").unwrap());
        prop_assert_eq!(q < ctrl, letters < bottom);
    }

    /// Keymaps from any arrangement can be pressed everywhere.
    #[test]
    fn arranged_keymaps_work_at_every_key(
        arrangement in arrangement(),
        taps in prop::collection::vec((0..3usize, 0..8usize, 0..14usize), 0..40),
    ) {
        let mut kb = Keyboard::new(Layout::builtin().keymap(&arrangement));
        for (layer, row, col) in taps {
            kb.tap(KeyId { layer, row, col });
        }
        prop_assert!(!kb.layer().rows.is_empty());
    }

    /// Plain keys tap exactly once per press, in press order.
    #[test]
    fn plain_keys_tap_once_in_order(cols in prop::collection::vec(0..10usize, 1..30)) {
        let mut gestures = Gestures::default();
        let mut taps = Vec::new();
        for (i, col) in cols.iter().enumerate() {
            let now = Duration::from_millis(i as u64 * 20);
            // Overlapping fingers: each press lifts only after the next goes down.
            taps.extend(gestures.press(i as i64, key(*col), PressKind::default(), 0.0, now));
            if i > 0 {
                taps.extend(gestures.release(i as i64 - 1, now));
            }
        }
        let expected: Vec<_> = cols.iter().map(|c| Trigger::Tap(key(*c))).collect();
        prop_assert_eq!(taps, expected);
    }

    /// The text buffer's cursor and selection always stay inside the text and on
    /// character boundaries, and its view always adds up to the text.
    #[test]
    fn text_buffer_stays_consistent(ops in prop::collection::vec(buffer_op(), 0..80)) {
        let mut buffer = TextBuffer::default();
        for op in ops {
            match op {
                BufferOp::Insert(text) => buffer.insert(&text),
                BufferOp::Backspace => buffer.backspace(),
                BufferOp::Delete => buffer.delete(),
                BufferOp::Move(motion, extend) => buffer.move_cursor(motion, extend),
            }
            let text = buffer.text();
            prop_assert!(text.is_char_boundary(buffer.cursor()));
            if let Some(range) = buffer.selection() {
                prop_assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
            }
            let view = buffer.view();
            prop_assert_eq!(format!("{}{}{}", view.before, view.selected, view.after), text);
        }
    }
}

/// Random arrangements of the built-in layout's sections, with some unknown
/// names mixed in.
fn arrangement() -> impl Strategy<Value = Arrangement> {
    let mut names: Vec<String> = Layout::builtin()
        .section_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    names.push("not-a-section".into());
    let order = proptest::sample::subsequence(names.clone(), 0..=names.len()).prop_shuffle();
    let enabled = prop::collection::btree_map(proptest::sample::select(names), any::<bool>(), 0..8);
    (order, enabled).prop_map(|(order, enabled)| Arrangement { order, enabled })
}

fn key(col: usize) -> KeyId {
    KeyId {
        layer: 0,
        row: 0,
        col,
    }
}

#[derive(Debug, Clone)]
struct GestureEvent {
    pointer: i64,
    after_ms: u64,
    kind: EventKind,
}

#[derive(Debug, Clone)]
enum EventKind {
    Press { kind: PressKind, col: usize, x: f32 },
    Move(f32),
    Release,
    Cancel,
    Tick,
}

fn gesture_event() -> impl Strategy<Value = GestureEvent> {
    let kind =
        (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(repeat, hold, swipe)| PressKind {
            repeat: repeat && !hold && !swipe,
            hold,
            swipe,
        });
    let event = prop_oneof![
        (kind, 0..10usize, -200.0f32..200.0).prop_map(|(kind, col, x)| EventKind::Press {
            kind,
            col,
            x
        }),
        (-400.0f32..400.0).prop_map(EventKind::Move),
        Just(EventKind::Release),
        Just(EventKind::Cancel),
        Just(EventKind::Tick),
    ];
    (0..4i64, 0..600u64, event).prop_map(|(pointer, after_ms, kind)| GestureEvent {
        pointer,
        after_ms,
        kind,
    })
}

#[derive(Debug, Clone)]
enum BufferOp {
    Insert(String),
    Backspace,
    Delete,
    Move(Motion, bool),
}

fn buffer_op() -> impl Strategy<Value = BufferOp> {
    let motion = proptest::sample::select(vec![
        Motion::Left,
        Motion::Right,
        Motion::Up,
        Motion::Down,
        Motion::WordLeft,
        Motion::WordRight,
        Motion::LineStart,
        Motion::LineEnd,
        Motion::Start,
        Motion::End,
    ]);
    prop_oneof![
        "[a-z é→_(){}\n\t]{0,6}".prop_map(BufferOp::Insert),
        Just(BufferOp::Backspace),
        Just(BufferOp::Delete),
        (motion, any::<bool>()).prop_map(|(m, extend)| BufferOp::Move(m, extend)),
    ]
}

/// Random layout files: mostly valid ones, with the occasional mistake the
/// parser must reject cleanly.
fn layout_source() -> impl Strategy<Value = String> {
    let short = prop_oneof![
        8 => proptest::sample::select(vec![
            "a", "->", "<esc>", "<bksp>", "<space>", "<ctrl>", "<shift>", "<f3>", "<", "<>",
            "<layer:base>",
        ]),
        1 => proptest::sample::select(vec!["<nope>", "", "<layer:missing>", "<layer:sym>"]),
    ]
    .prop_map(|s| format!("{s:?}"));
    let table = (
        proptest::sample::select(vec!["x", "<left>", "<space>", "<layer:base>"]),
        proptest::option::of(proptest::sample::select(vec![
            "..",
            "<home>",
            "<layer:base>",
        ])),
        proptest::option::of(prop_oneof![9 => Just("cursor"), 1 => Just("zoom")]),
        proptest::option::of(any::<bool>()),
        proptest::option::of(prop_oneof![9 => 0.5f32..4.0, 1 => -1.0f32..0.5]),
    )
        .prop_map(|(tap, hold, swipe, repeat, width)| {
            let mut fields = vec![format!("tap = {tap:?}")];
            fields.extend(hold.map(|h| format!("hold = {h:?}")));
            fields.extend(swipe.map(|s| format!("swipe = {s:?}")));
            fields.extend(repeat.map(|r| format!("repeat = {r}")));
            fields.extend(width.map(|w| format!("width = {w:?}")));
            format!("{{ {} }}", fields.join(", "))
        });
    let key = prop_oneof![3 => short, 1 => table];
    let row = prop::collection::vec(key, 1..6).prop_map(|keys| format!("[{}]", keys.join(", ")));
    let rows = prop::collection::vec(row, 1..4).prop_map(|rows| rows.join(", "));
    // Distinct layer names, always starting with `base`.
    let names = proptest::sample::subsequence(vec!["sym", "nav", "fn"], 0..=3);
    (names, prop::collection::vec(rows, 4)).prop_map(|(names, rows)| {
        let layers: Vec<String> = std::iter::once("base")
            .chain(names)
            .zip(rows)
            .map(|(name, rows)| format!("[[layers]]\nname = {name:?}\nrows = [{rows}]\n"))
            .collect();
        format!("name = \"random\"\n{}", layers.join("\n"))
    })
}

#[test]
fn layout_strategy_produces_valid_layouts_too() {
    use proptest::{strategy::ValueTree, test_runner::TestRunner};
    let mut runner = TestRunner::deterministic();
    let valid = (0..200)
        .filter(|_| {
            let source = layout_source().new_tree(&mut runner).unwrap().current();
            Layout::from_toml(&source).is_ok()
        })
        .count();
    // About a quarter is valid: enough for the property above to exercise real
    // layouts on every run, while still feeding the parser mistakes.
    assert!(valid > 25, "only {valid} of 200 random layouts were valid");
}

#[test]
fn modifier_and_layer_keys_produce_no_output() {
    let mut kb = Keyboard::new(Keymap::builtin());
    for id in builtin_ids() {
        let tap = kb.key(id).unwrap().tap.clone();
        if is_modifier_or_layer(&tap) {
            assert_eq!(kb.tap(id), None, "{tap:?}");
        }
    }
}

proptest! {
    /// With auto-pairing on, the text before the cursor is exactly what was
    /// typed, and all that is ever added after the cursor is closers.
    #[test]
    fn auto_pair_keeps_what_was_typed_before_the_cursor(
        keys in "[a-c0-9 ,;()\\[\\]{}\"'`]{0,40}",
    ) {
        let mut pairs = AutoPair::new(true);
        let mut sink = PreviewSink::default();
        for key in keys.chars() {
            let output = match key {
                ' ' => Output::named(NamedKey::Space, Modifiers::NONE),
                key => Output::Text(key.into()),
            };
            for output in pairs.process(output) {
                sink.emit(output);
            }
        }
        let view = sink.buffer().view();
        prop_assert_eq!(view.before, keys.as_str());
        prop_assert!(view.after.chars().all(|c| ")]}\"'`".contains(c)), "{:?}", view.after);
    }
}

/// A key for the Vietnamese property: a letter, a digit, VIQR punctuation, a
/// bracket, a space or Backspace.
fn vietnamese_key() -> impl Strategy<Value = Output> {
    prop_oneof![
        8 => proptest::sample::select(
            "aeiouyAEOdDswfrxjzWbcghknmptqv0123456789'`?~.^(+[]\\"
                .chars()
                .collect::<Vec<_>>()
        )
        .prop_map(|c| Output::Text(c.into())),
        1 => Just(Output::named(NamedKey::Space, Modifiers::NONE)),
        2 => Just(Output::named(NamedKey::Backspace, Modifiers::NONE)),
    ]
}

proptest! {
    /// Whatever is typed, the text on screen ends with the syllable the composer
    /// is working on: the Backspaces it sends only ever erase its own text.
    #[test]
    fn vietnamese_only_rewrites_its_own_syllable(
        keys in prop::collection::vec(vietnamese_key(), 0..80),
        method in proptest::sample::select(InputMethod::ALL.to_vec()),
        modern in any::<bool>(),
        check_spelling in any::<bool>(),
        telex_w in any::<bool>(),
        telex_brackets in any::<bool>(),
        nfd in any::<bool>(),
    ) {
        let mut composer = Composer::new(VietnameseOptions {
            enabled: true,
            method,
            tone_style: if modern { ToneStyle::Modern } else { ToneStyle::Classic },
            check_spelling,
            telex_w,
            telex_brackets,
            unicode_form: if nfd { UnicodeForm::Nfd } else { UnicodeForm::Nfc },
        });
        let mut sink = PreviewSink::default();
        sink.emit(Output::Text("> ".into()));
        let backspace = Output::named(NamedKey::Backspace, Modifiers::NONE);
        for key in keys {
            // The text before the syllable is only the user's to erase, with a
            // Backspace while no syllable is in progress.
            let composing = composer.composing();
            let fixed = sink.text().strip_suffix(composing.as_str()).unwrap().to_owned();
            let may_erase = key == backspace && composing.is_empty();
            for output in composer.process(key) {
                sink.emit(output);
            }
            let composing = composer.composing();
            prop_assert!(sink.text().ends_with(&composing), "{:?} / {composing:?}", sink.text());
            if !may_erase {
                prop_assert!(sink.text().starts_with(&fixed), "{:?} lost {fixed:?}", sink.text());
            }
        }
    }
}
