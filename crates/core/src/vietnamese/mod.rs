//! Vietnamese typing with Telex or VNI.
//!
//! A [`Composer`] sits between the [`Keyboard`](crate::Keyboard) and an
//! [`OutputSink`](crate::OutputSink). It keeps the syllable being typed and,
//! for each key, sends the change to what is already on screen: Backspaces for
//! the characters that changed, then the new text. `V`, `i`, `e`, `e` types
//! `V`, `i`, `e`, then ⌫ and `ê`. This works with any sink that can insert
//! text and press Backspace, which is every sink.
//!
//! | Telex | VNI | Result |
//! |---|---|---|
//! | `s` `f` `r` `x` `j` | `1` `2` `3` `4` `5` | sắc á, huyền à, hỏi ả, ngã ã, nặng ạ |
//! | `z` | `0` | remove the tone |
//! | `aa` `ee` `oo` | `a6` `e6` `o6` | â ê ô |
//! | `aw` | `a8` | ă |
//! | `ow` `uw` (or `w`) | `o7` `u7` | ơ ư |
//! | `dd` | `d9` | đ |
//!
//! Marks can be typed anywhere in the syllable (`tieengs` and `tiengse` are
//! both `tiếng`), and the tone moves to the right vowel as the syllable grows.
//! Typing a mark's key twice takes it off and types the key (`ass` is `as`).
//! With [`VietnameseOptions::check_spelling`], keys only change syllables that
//! can be Vietnamese, so most English words and code type as written.
//!
//! Letters with marks are written precomposed (NFC: `ệ` is one character)
//! unless [`VietnameseOptions::unicode_form`] asks for decomposed text (NFD:
//! `e` and two combining marks), for the rare application that needs it.

mod letters;
mod word;

use crate::{
    action::{Modifiers, NamedKey},
    output::{KeyCode, Output, OutputSink, PreviewSink},
};
use word::Word;

/// How keys spell marks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum InputMethod {
    /// Letters: `aa` is â, `s` is sắc.
    #[default]
    Telex,
    /// Digits: `a6` is â, `1` is sắc.
    Vni,
}

impl InputMethod {
    pub const ALL: [Self; 2] = [Self::Telex, Self::Vni];

    /// The name used in `settings.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Telex => "telex",
            Self::Vni => "vni",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|method| method.name() == name)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Telex => "Telex",
            Self::Vni => "VNI",
        }
    }
}

/// Which vowel of `oa`, `oe` and `uy` takes the tone when nothing follows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ToneStyle {
    /// The first: hòa, khỏe, thủy.
    #[default]
    Classic,
    /// The second: hoà, khoẻ, thuỷ.
    Modern,
}

impl ToneStyle {
    pub const ALL: [Self; 2] = [Self::Classic, Self::Modern];

    /// The name used in `settings.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Modern => "modern",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.name() == name)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Modern => "Modern",
        }
    }
}

/// How letters with marks are written in Unicode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UnicodeForm {
    /// Precomposed: `ệ` is one character. What almost every application
    /// expects.
    #[default]
    Nfc,
    /// Decomposed: `ệ` is `e` followed by its combining marks.
    Nfd,
}

impl UnicodeForm {
    pub const ALL: [Self; 2] = [Self::Nfc, Self::Nfd];

    /// The name used in `settings.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Nfc => "nfc",
            Self::Nfd => "nfd",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|form| form.name() == name)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Nfc => "Precomposed (NFC)",
            Self::Nfd => "Decomposed (NFD)",
        }
    }
}

/// The `[vietnamese]` settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VietnameseOptions {
    /// Off, keys type exactly what they show.
    pub enabled: bool,
    pub method: InputMethod,
    pub tone_style: ToneStyle,
    /// Only change syllables that can be Vietnamese.
    pub check_spelling: bool,
    /// Write letters with marks precomposed or decomposed.
    pub unicode_form: UnicodeForm,
}

impl Default for VietnameseOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            method: InputMethod::default(),
            tone_style: ToneStyle::default(),
            check_spelling: true,
            unicode_form: UnicodeForm::default(),
        }
    }
}

/// Turns typed keys into Vietnamese as they arrive. See the [module
/// docs](self).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Composer {
    options: VietnameseOptions,
    word: Word,
}

impl Composer {
    pub fn new(options: VietnameseOptions) -> Self {
        Self {
            options,
            word: Word::default(),
        }
    }

    pub fn options(&self) -> VietnameseOptions {
        self.options
    }

    /// Changes the options. The syllable in progress is finished as it is.
    pub fn set_options(&mut self, options: VietnameseOptions) {
        if options != self.options {
            self.options = options;
            self.word.clear();
        }
    }

    /// Forgets the syllable in progress, for when the text it was typed into
    /// changed some other way.
    pub fn reset(&mut self) {
        self.word.clear();
    }

    /// The syllable being typed, as shown; empty between words. It is written
    /// in [`VietnameseOptions::unicode_form`].
    pub fn composing(&self) -> String {
        let text = self.word.render(self.options.tone_style);
        match self.options.unicode_form {
            UnicodeForm::Nfc => text,
            UnicodeForm::Nfd => {
                let mut decomposed = String::with_capacity(text.len() * 2);
                for c in text.chars() {
                    letters::decompose(c, &mut decomposed);
                }
                decomposed
            }
        }
    }

    /// Adds a typed character to the syllable, as an input method that shows
    /// the syllable as pre-edit text does. Returns false when `key` is not part
    /// of a syllable (a space, punctuation, a Telex digit, or a VNI digit with
    /// nothing to mark): the caller then finishes the syllable with
    /// [`take`](Self::take) and handles `key` itself. Works whether or not
    /// [`VietnameseOptions::enabled`] is set; checking it is the caller's job.
    pub fn push_key(&mut self, key: char) -> bool {
        self.is_word_key(key) && self.word.push_key(key, &self.options)
    }

    /// Takes the last letter off the syllable, with its marks. Returns false
    /// when there is no syllable, so the Backspace belongs to the application.
    pub fn backspace(&mut self) -> bool {
        if self.word.is_empty() {
            return false;
        }
        self.word.pop(self.options.tone_style);
        true
    }

    /// Finishes the syllable: returns it as shown, and starts a new one.
    pub fn take(&mut self) -> String {
        let text = self.composing();
        self.word.clear();
        text
    }

    /// What to send for `output`. Anything that is not a letter (or a VNI
    /// digit) ends the syllable and passes through unchanged, and so does
    /// everything while Vietnamese is off.
    pub fn process(&mut self, output: Output) -> Vec<Output> {
        if !self.options.enabled {
            return vec![output];
        }
        match &output {
            Output::Text(text) => {
                let mut chars = text.chars();
                if let (Some(key), None) = (chars.next(), chars.next()) {
                    let before = self.composing();
                    if self.push_key(key) {
                        return diff(&before, &self.composing());
                    }
                }
            }
            Output::Key {
                key: KeyCode::Named(NamedKey::Backspace),
                mods,
            } if mods.is_empty() => {
                let before = self.composing();
                if self.backspace() {
                    return diff(&before, &self.composing());
                }
            }
            Output::Key { .. } => {}
        }
        self.word.clear();
        vec![output]
    }

    fn is_word_key(&self, key: char) -> bool {
        match self.options.method {
            InputMethod::Telex => key.is_ascii_alphabetic(),
            InputMethod::Vni => key.is_ascii_alphanumeric(),
        }
    }
}

/// Backspaces over what changed between `before` and `after`, then the rest
/// of `after`. One Backspace erases one code point, which in decomposed text
/// is a letter or a single mark.
fn diff(before: &str, after: &str) -> Vec<Output> {
    let common = before
        .chars()
        .zip(after.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let erase = before.chars().count() - common;
    let mut outputs = vec![Output::named(NamedKey::Backspace, Modifiers::NONE); erase];
    let rest: String = after.chars().skip(common).collect();
    if !rest.is_empty() {
        outputs.push(Output::Text(rest));
    }
    outputs
}

/// Types `keys` with `options` (as if Vietnamese were on) and returns the
/// text. Spaces and newlines are sent as the Space and Enter keys. Settings
/// screens use it to show what keys make.
pub fn transliterate(keys: &str, options: VietnameseOptions) -> String {
    let mut composer = Composer::new(VietnameseOptions {
        enabled: true,
        ..options
    });
    let mut sink = PreviewSink::default();
    for key in keys.chars() {
        let output = match key {
            ' ' => Output::named(NamedKey::Space, Modifiers::NONE),
            '\n' => Output::named(NamedKey::Enter, Modifiers::NONE),
            _ => Output::Text(key.into()),
        };
        for output in composer.process(output) {
            sink.emit(output);
        }
    }
    sink.text().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Modifier;

    fn telex() -> VietnameseOptions {
        VietnameseOptions::default()
    }

    fn vni() -> VietnameseOptions {
        VietnameseOptions {
            method: InputMethod::Vni,
            ..VietnameseOptions::default()
        }
    }

    fn modern(options: VietnameseOptions) -> VietnameseOptions {
        VietnameseOptions {
            tone_style: ToneStyle::Modern,
            ..options
        }
    }

    fn nfd(options: VietnameseOptions) -> VietnameseOptions {
        VietnameseOptions {
            unicode_form: UnicodeForm::Nfd,
            ..options
        }
    }

    fn check(options: VietnameseOptions, cases: &[(&str, &str)]) {
        for (keys, expected) in cases {
            assert_eq!(
                transliterate(keys, options),
                *expected,
                "{keys} with {options:?}"
            );
        }
    }

    #[test]
    fn telex_sentences() {
        check(
            telex(),
            &[
                ("Tieesng Vieejt", "Tiếng Việt"),
                ("Vieejt Nam", "Việt Nam"),
                ("Xin chaof cacs banj", "Xin chào các bạn"),
                ("Tooi yeeu tieesng Vieejt", "Tôi yêu tiếng Việt"),
                ("dduwowngf phoos", "đường phố"),
                ("nguwowif", "người"),
                (
                    "Khoong cos gif quys hown ddoocj laapj tuwj do",
                    "Không có gì quý hơn độc lập tự do",
                ),
                ("hocj taapj", "học tập"),
                ("xuaan", "xuân"),
                ("hoawcj", "hoặc"),
                ("muwa", "mưa"),
                ("cuwux", "cữu"),
                ("tuooir", "tuổi"),
                ("khuyeen", "khuyên"),
                ("khuyar", "khuỷa"),
                ("gias", "giá"),
                ("gif", "gì"),
                ("quas", "quá"),
                ("quoocs", "quốc"),
                ("giuwax", "giữa"),
            ],
        );
    }

    #[test]
    fn telex_marks_go_anywhere_in_the_syllable() {
        check(
            telex(),
            &[
                ("tiengse", "tiếng"),
                ("tieengs", "tiếng"),
                ("nguoiwf", "người"),
                ("nguoifw", "người"),
                ("tana", "tân"),
                ("did", "đi"),
                ("hoaf", "hòa"),
                ("hoafn", "hoàn"),
                ("hoanf", "hoàn"),
            ],
        );
    }

    #[test]
    fn telex_lone_w_and_capitals() {
        check(
            telex(),
            &[
                ("w", "ư"),
                ("tw", "tư"),
                ("nhwng", "nhưng"),
                ("W", "Ư"),
                ("ww", "w"),
                ("TIEENGS VIEEJT", "TIẾNG VIỆT"),
                ("DDaay", "Đây"),
                ("NGUWOWIF", "NGƯỜI"),
                ("Aas", "Ấ"),
            ],
        );
    }

    #[test]
    fn a_second_mark_key_takes_the_mark_off() {
        check(
            telex(),
            &[
                ("ass", "as"),
                ("asss", "ass"),
                ("aaa", "aa"),
                ("aww", "aw"),
                ("ddd", "dd"),
                ("asz", "a"),
                ("az", "az"),
                ("asf", "à"),
            ],
        );
        check(vni(), &[("a11", "a1"), ("a66", "a6"), ("d99", "d9")]);
    }

    #[test]
    fn tone_style_moves_the_tone_in_oa_oe_and_uy() {
        let cases = [
            ("hoaf", "hòa", "hoà"),
            ("khoer", "khỏe", "khoẻ"),
            ("thuyr", "thủy", "thuỷ"),
            ("quys", "quý", "quý"),
            ("hoanf", "hoàn", "hoàn"),
            ("muaf", "mùa", "mùa"),
        ];
        for (keys, classic, modern_form) in cases {
            assert_eq!(transliterate(keys, telex()), classic, "{keys}");
            assert_eq!(transliterate(keys, modern(telex())), modern_form, "{keys}");
        }
    }

    #[test]
    fn nfd_writes_each_letter_then_its_marks() {
        let cases = [
            ("Vieejt", "Vie\u{323}\u{302}t"),
            ("dduwowngf", "đu\u{31b}o\u{31b}\u{300}ng"),
            ("hoawcj", "hoa\u{323}\u{306}c"),
            ("AAs", "A\u{302}\u{301}"),
            ("Dday", "Đay"),
            ("class", "class"),
        ];
        check(nfd(telex()), &cases);
        check(nfd(vni()), &[("Vie65t", "Vie\u{323}\u{302}t")]);
        // NFC, the default, writes the same words with one character a letter.
        check(telex(), &[("Vieejt", "Việt"), ("hoawcj", "hoặc")]);
    }

    #[test]
    fn nfd_erases_one_code_point_per_backspace() {
        let mut composer = Composer::new(VietnameseOptions {
            enabled: true,
            ..nfd(telex())
        });
        let bksp = Output::named(NamedKey::Backspace, Modifiers::NONE);
        let text = |s: &str| Output::Text(s.into());
        assert_eq!(composer.process(text("e")), [text("e")]);
        // A mark that goes last is only added.
        assert_eq!(composer.process(text("e")), [text("\u{302}")]);
        // The dot below sorts before the circumflex, so the circumflex is retyped.
        assert_eq!(
            composer.process(text("j")),
            [bksp.clone(), text("\u{323}\u{302}")]
        );
        assert_eq!(composer.composing(), "e\u{323}\u{302}");
        // Backspace takes the whole letter: three code points.
        assert_eq!(
            composer.process(bksp.clone()),
            [bksp.clone(), bksp.clone(), bksp.clone()]
        );
        assert_eq!(composer.composing(), "");
    }

    #[test]
    fn vni_sentences() {
        check(
            vni(),
            &[
                ("Tie61ng Vie65t", "Tiếng Việt"),
                ("Vie65t Nam", "Việt Nam"),
                ("d9u7o7ng2", "đường"),
                ("nguo72i", "người"),
                ("ngu7o72i", "người"),
                ("hoa2", "hòa"),
                ("hoa85c", "hoặc"),
                ("mu7a", "mưa"),
                ("xua6n", "xuân"),
            ],
        );
    }

    #[test]
    fn vni_digits_with_nothing_to_mark_stay_digits() {
        check(
            vni(),
            &[
                ("123", "123"),
                ("x2", "x2"),
                ("v8", "v8"),
                ("a0", "a0"),
                ("a1b2", "áb2"),
            ],
        );
    }

    #[test]
    fn spelling_keeps_english_and_code_as_typed() {
        check(
            telex(),
            &[
                ("for", "for"),
                ("class", "class"),
                ("fix", "fix"),
                ("string", "string"),
                ("print", "print"),
            ],
        );
        let loose = VietnameseOptions {
            check_spelling: false,
            ..telex()
        };
        assert_eq!(transliterate("for", loose), "fỏ");
    }

    #[test]
    fn sends_backspaces_and_the_changed_text() {
        let mut composer = Composer::new(VietnameseOptions {
            enabled: true,
            ..telex()
        });
        let bksp = Output::named(NamedKey::Backspace, Modifiers::NONE);
        let text = |s: &str| Output::Text(s.into());
        assert_eq!(composer.process(text("h")), [text("h")]);
        assert_eq!(composer.process(text("o")), [text("o")]);
        assert_eq!(composer.process(text("a")), [text("a")]);
        assert_eq!(
            composer.process(text("f")),
            [bksp.clone(), bksp.clone(), text("òa")]
        );
        assert_eq!(composer.composing(), "hòa");
        assert_eq!(
            composer.process(text("n")),
            [bksp.clone(), bksp.clone(), text("oàn")]
        );
        // Backspace takes the n off, and the tone moves back.
        assert_eq!(
            composer.process(bksp.clone()),
            [bksp.clone(), bksp.clone(), bksp.clone(), text("òa")]
        );
    }

    #[test]
    fn other_output_ends_the_syllable() {
        let mut composer = Composer::new(VietnameseOptions {
            enabled: true,
            ..telex()
        });
        let mut sink = PreviewSink::default();
        let mut send = |composer: &mut Composer, output: Output| {
            for output in composer.process(output) {
                sink.emit(output);
            }
        };
        for key in ["a", ".", "s", "a"] {
            send(&mut composer, Output::Text(key.into()));
        }
        send(
            &mut composer,
            Output::named(NamedKey::Left, Modifiers::NONE),
        );
        send(&mut composer, Output::Text("s".into()));
        // Ctrl+S is a chord for the app, not a tone.
        send(
            &mut composer,
            Output::Key {
                key: KeyCode::Char('s'),
                mods: Modifiers::NONE.with(Modifier::Ctrl),
            },
        );
        send(&mut composer, Output::Text("->".into()));
        assert_eq!(sink.text(), "a.ss->a");
        assert_eq!(composer.composing(), "");
    }

    #[test]
    fn off_passes_everything_through() {
        let mut composer = Composer::default();
        assert!(!composer.options().enabled);
        assert_eq!(
            composer.process(Output::Text("s".into())),
            [Output::Text("s".into())]
        );
        assert_eq!(composer.composing(), "");
    }

    #[test]
    fn changing_options_finishes_the_syllable() {
        let mut composer = Composer::new(VietnameseOptions {
            enabled: true,
            ..telex()
        });
        composer.process(Output::Text("a".into()));
        composer.set_options(VietnameseOptions {
            enabled: true,
            ..vni()
        });
        assert_eq!(composer.composing(), "");
        assert_eq!(
            composer.process(Output::Text("s".into())),
            [Output::Text("s".into())]
        );
    }

    #[test]
    fn names_round_trip() {
        for method in InputMethod::ALL {
            assert_eq!(InputMethod::from_name(method.name()), Some(method));
        }
        for style in ToneStyle::ALL {
            assert_eq!(ToneStyle::from_name(style.name()), Some(style));
        }
        for form in UnicodeForm::ALL {
            assert_eq!(UnicodeForm::from_name(form.name()), Some(form));
        }
        assert_eq!(InputMethod::from_name("Telex"), None);
        assert_eq!(UnicodeForm::from_name("NFD"), None);
    }
}
