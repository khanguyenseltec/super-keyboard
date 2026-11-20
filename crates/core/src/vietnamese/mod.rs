//! Vietnamese typing with Telex, VNI or VIQR.
//!
//! A [`Composer`] sits between the [`Keyboard`](crate::Keyboard) and an
//! [`OutputSink`](crate::OutputSink). It keeps the syllable being typed and,
//! for each key, sends the change to what is already on screen: Backspaces for
//! the characters that changed, then the new text. `V`, `i`, `e`, `e` types
//! `V`, `i`, `e`, then ⌫ and `ê`. This works with any sink that can insert
//! text and press Backspace, which is every sink.
//!
//! | Telex | VNI | VIQR | Result |
//! |---|---|---|---|
//! | `s` `f` `r` `x` `j` | `1` `2` `3` `4` `5` | `'` `` ` `` `?` `~` `.` | sắc á, huyền à, hỏi ả, ngã ã, nặng ạ |
//! | `z` | `0` | `0` | remove the tone |
//! | `aa` `ee` `oo` | `a6` `e6` `o6` | `a^` `e^` `o^` | â ê ô |
//! | `aw` | `a8` | `a(` | ă |
//! | `ow` `uw` (or `w`) | `o7` `u7` | `o+` `u+` | ơ ư |
//! | `dd` | `d9` | `dd` | đ |
//!
//! Telex has two options: [`VietnameseOptions::telex_w`] (a `w` with no vowel
//! to mark is `ư`; on by default) and [`VietnameseOptions::telex_brackets`]
//! (`[` and `]` type `ơ` and `ư`; off by default).
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
    /// Punctuation: `a^` is â, `'` is sắc.
    Viqr,
}

impl InputMethod {
    pub const ALL: [Self; 3] = [Self::Telex, Self::Vni, Self::Viqr];

    /// The name used in `settings.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Telex => "telex",
            Self::Vni => "vni",
            Self::Viqr => "viqr",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|method| method.name() == name)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Telex => "Telex",
            Self::Vni => "VNI",
            Self::Viqr => "VIQR",
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
    /// Telex: a `w` with no vowel to mark types `ư` (`tw` is `tư`). Off, it
    /// types `w`, and `ư` is `uw`.
    pub telex_w: bool,
    /// Telex: `[` types `ơ` and `]` types `ư`.
    pub telex_brackets: bool,
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
            telex_w: true,
            telex_brackets: false,
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
    /// of a syllable (a space, punctuation, a Telex digit, or a VNI digit,
    /// VIQR mark or Telex bracket with nothing to mark): the caller then
    /// finishes the syllable with
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

    /// What to send for `output`. Anything that is not a letter (or a key the
    /// method marks with: a VNI digit, VIQR punctuation, a Telex bracket) ends
    /// the syllable and passes through unchanged, and so does everything while
    /// Vietnamese is off.
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

    /// Letters, and the other keys the method marks with: VNI digits, VIQR
    /// punctuation and `\\`, Telex brackets when they are on.
    fn is_word_key(&self, key: char) -> bool {
        key.is_ascii_alphabetic() || word::Effect::of(&self.options, key).is_some()
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

    fn viqr() -> VietnameseOptions {
        VietnameseOptions {
            method: InputMethod::Viqr,
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
    fn viqr_sentences() {
        check(
            viqr(),
            &[
                ("Tie^'ng Vie^.t", "Tiếng Việt"),
                ("Vie^.t Nam", "Việt Nam"),
                ("Xin cha`o ca'c ba.n", "Xin chào các bạn"),
                ("ddu+o+`ng pho^'", "đường phố"),
                ("nguo+`i", "người"),
                ("ngu+o+`i", "người"),
                ("ho?i nga~", "hỏi ngã"),
                ("hoa(.c", "hoặc"),
                ("mu+a", "mưa"),
                ("xua^n", "xuân"),
                ("DDa^y", "Đây"),
                ("TIE^'NG", "TIẾNG"),
            ],
        );
    }

    #[test]
    fn viqr_marks_go_anywhere_and_come_off() {
        check(
            viqr(),
            &[
                ("tieng^'", "tiếng"),
                ("hoa`n", "hoàn"),
                ("hoan`", "hoàn"),
                ("a''", "a'"),
                ("a^^", "a^"),
                ("a((", "a("),
                ("o++", "o+"),
                ("ddd", "dd"),
                ("a'0", "a"),
                ("a'`", "à"),
            ],
        );
    }

    #[test]
    fn viqr_punctuation_with_nothing_to_mark_stays_punctuation() {
        check(
            viqr(),
            &[
                ("x.", "x."),
                ("...", "..."),
                ("f(x)", "f(x)"),
                ("c++", "c++"),
                ("i++", "i++"),
                ("a0", "a0"),
                ("3.5", "3.5"),
                ("'a'", "'á"),
                ("hoc`", "hoc`"),
                ("ba. ", "bạ "),
                ("ba.. ", "ba. "),
                ("sao?? ", "sao? "),
            ],
        );
    }

    #[test]
    fn viqr_sentence_ends_keep_the_tone() {
        check(
            viqr(),
            &[
                // A tone the syllable already has: the key is punctuation.
                ("To^i ye^u tie^'ng Vie^.t.", "Tôi yêu tiếng Việt."),
                ("ho.c. ", "học. "),
                ("Kho?e?", "Khỏe?"),
                ("Vie^.t..", "Việt.."),
                // Another tone: the key changes it, and again at once takes
                // it back and is typed.
                ("ba.n?", "bản"),
                ("ba.n??", "bạn?"),
                ("Ba.n kho?e kho^ng??", "Bạn khỏe không?"),
                ("Xin chao..", "Xin chao."),
            ],
        );
    }

    #[test]
    fn viqr_backslash_types_a_mark_as_itself() {
        check(
            viqr(),
            &[
                ("kho^ng\\?", "không?"),
                ("Vie^.t\\.", "Việt."),
                ("a\\'", "a'"),
                ("\\\\", "\\"),
                // Before anything else, the backslash stays.
                ("a\\b", "a\\b"),
                ("C:\\x", "C:\\x"),
            ],
        );
    }

    #[test]
    fn viqr_puts_back_what_is_not_vietnamese() {
        check(
            viqr(),
            &[
                ("don't", "don't"),
                ("let's go", "let's go"),
                ("can't", "can't"),
                ("ma.com", "ma.com"),
                ("e.g.", "e.g."),
                ("a(b)", "a(b)"),
                ("ba.n.com", "ba.n.com"),
                ("Vie^.t", "Việt"),
                // Still a Vietnamese syllable, so it stays one: type `I\'m`.
                ("I'm", "Ím"),
                ("I\\'m", "I'm"),
            ],
        );
        // Without the spelling check nothing is put back.
        let loose = VietnameseOptions {
            check_spelling: false,
            ..viqr()
        };
        check(loose, &[("don't", "dónt")]);
    }

    #[test]
    fn telex_brackets_follow_the_case_and_give_back_code() {
        let brackets = VietnameseOptions {
            telex_brackets: true,
            ..telex()
        };
        check(
            brackets,
            &[
                ("TH[", "THƠ"),
                ("NG][IF", "NGƯỜI"),
                ("Th[", "Thơ"),
                ("T[", "Tơ"),
                ("m[k]", "m[k]"),
                // `xơ` is a syllable, so indexing needs brackets off or `[[`.
                ("x[0]", "xơ0ư"),
                ("x[[0]]", "x[0]"),
            ],
        );
    }

    #[test]
    fn a_reverted_letter_takes_its_tone_with_it() {
        let type_then = |options: VietnameseOptions, keys: &str, after: &str| {
            let mut composer = Composer::new(VietnameseOptions {
                enabled: true,
                ..options
            });
            for key in keys.chars() {
                composer.push_key(key);
            }
            composer.backspace();
            for key in after.chars() {
                composer.push_key(key);
            }
            composer.composing()
        };
        let brackets = VietnameseOptions {
            telex_brackets: true,
            ..telex()
        };
        assert_eq!(type_then(brackets, "th[f[", "a"), "tha");
        assert_eq!(type_then(telex(), "nhwfw", "a"), "nha");
    }

    /// Types `keys`, where `<` is Backspace, and returns the syllable.
    fn compose(options: VietnameseOptions, keys: &str) -> String {
        let mut composer = Composer::new(VietnameseOptions {
            enabled: true,
            ..options
        });
        for key in keys.chars() {
            if key == '<' {
                composer.backspace();
            } else {
                composer.push_key(key);
            }
        }
        composer.composing()
    }

    #[test]
    fn putting_back_never_loses_what_came_before_a_backspace() {
        // After a Backspace the word is not put back as typed, so nothing
        // typed earlier can be lost.
        assert_eq!(compose(viqr(), "ban<'x"), "báx");
        assert_eq!(compose(viqr(), "cann<'t"), "cánt");
        assert_eq!(compose(viqr(), "dx<on't"), "dónt");
        let brackets = VietnameseOptions {
            telex_brackets: true,
            ..telex()
        };
        assert_eq!(compose(brackets, "mx<[k"), "mơk");
        assert_eq!(compose(brackets, "th<h[k"), "thơk");
    }

    #[test]
    fn putting_back_does_not_repeat_an_undone_key() {
        check(viqr(), &[("ba''x", "ba'x"), ("ba.n??x", "bạn?x")]);
        let brackets = VietnameseOptions {
            telex_brackets: true,
            ..telex()
        };
        // `[[` is a bracket; the `]` after a bracket word types as written.
        check(brackets, &[("m[[k", "m[k"), ("t[[i]", "t[i]")]);
    }

    #[test]
    fn a_backslash_only_escapes_a_mark_that_would_apply() {
        check(
            viqr(),
            &[
                ("C:\\dev", "C:\\dev"),
                ("C:\\Users\\dan", "C:\\Users\\dan"),
                ("\\(x\\)", "\\(x\\)"),
                ("x\\+y", "x\\+y"),
                ("\\0", "\\0"),
                ("a\\.b", "a.b"),
            ],
        );
        // A Backspace over the backslash cancels it.
        assert_eq!(compose(viqr(), "a\\<?"), "ả");
    }

    #[test]
    fn a_reverted_lone_w_takes_its_tone_with_it() {
        check(telex(), &[("nhwfwa", "nhwa")]);
    }

    #[test]
    fn telex_and_vni_leave_viqr_punctuation_alone() {
        check(telex(), &[("a'", "a'"), ("a.", "a."), ("o+", "o+")]);
        check(vni(), &[("a'", "a'"), ("a^", "a^"), ("a(", "a(")]);
    }

    #[test]
    fn telex_without_the_lone_w() {
        let plain_w = VietnameseOptions {
            telex_w: false,
            ..telex()
        };
        check(
            plain_w,
            &[
                ("w", "w"),
                ("tw", "tw"),
                ("W", "W"),
                ("ww", "ww"),
                ("nhwng", "nhwng"),
                // `w` still marks a vowel that is there.
                ("tuw", "tư"),
                ("nguwowif", "người"),
                ("hoawcj", "hoặc"),
                ("uww", "uw"),
            ],
        );
        check(telex(), &[("w", "ư"), ("tw", "tư")]);
    }

    #[test]
    fn telex_brackets_type_o_and_u_with_a_horn() {
        let brackets = VietnameseOptions {
            telex_brackets: true,
            ..telex()
        };
        check(
            brackets,
            &[
                ("[", "ơ"),
                ("]", "ư"),
                ("th[", "thơ"),
                ("m]a", "mưa"),
                ("ng][if", "người"),
                ("dd][ngf", "đường"),
                ("t]j", "tự"),
                // Twice gives the bracket back, like `ww`.
                ("[[", "["),
                ("]]", "]"),
                ("th[[", "th["),
                // Not a syllable: the bracket is a bracket.
                ("a[", "a["),
                ("[]", "ơ]"),
                ("{", "{"),
            ],
        );
        // Off, as it is by default, brackets are never letters.
        check(telex(), &[("[", "["), ("th[", "th["), ("]]", "]]")]);
        // The option belongs to Telex.
        let vni_brackets = VietnameseOptions {
            telex_brackets: true,
            ..vni()
        };
        check(vni_brackets, &[("[", "["), ("th]", "th]")]);
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
