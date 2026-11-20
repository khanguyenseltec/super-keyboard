//! The syllable being typed: its letters, its tone, and the rules that change
//! them as keys arrive.
//!
//! The tone belongs to the syllable, not to a letter. Each time the syllable is
//! drawn, the tone goes on the vowel the spelling rules pick, so it moves when
//! the syllable grows: `hòa` + `n` is `hoàn`.

use std::ops::Range;

use super::{
    InputMethod, ToneStyle, VietnameseOptions,
    letters::{Letter, Mark, Tone, vowel_parts},
};

/// Consonants a syllable can start with. `qu` and `gi` take the `u` or `i`
/// when another vowel follows (`quá`, `giá`).
const INITIALS: [&str; 27] = [
    "", "b", "c", "ch", "d", "g", "gh", "gi", "h", "k", "kh", "l", "m", "n", "ng", "ngh", "nh",
    "p", "ph", "qu", "r", "s", "t", "th", "tr", "v", "x",
];

/// Consonants a syllable can end with.
const FINALS: [&str; 9] = ["", "c", "ch", "m", "n", "ng", "nh", "p", "t"];

/// Finals after which only the sắc and nặng tones (or none) are allowed.
const STOP_FINALS: [&str; 4] = ["c", "ch", "p", "t"];

/// Every vowel group of Vietnamese, with its shape marks. A group being typed
/// matches one if each letter is the same base vowel with the same mark or no
/// mark yet, so `uo` matches `uô`, `uơ` and `ươ` until a mark decides.
const VOWEL_GROUPS: [&str; 56] = [
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y", //
    "ai", "ao", "au", "âu", "ay", "ây", "eo", "êu", "ia", "iê", "iu", "oa", "oă", "oe", "oi", "ôi",
    "ơi", "oo", "ua", "uâ", "uê", "ui", "uô", "uơ", "uy", "ưa", "ưi", "ươ", "ưu", "yê", "iêu",
    "oai", "oao", "oay", "oeo", "uây", "uôi", "uya", "uyê", "uyu", "ươi", "ươu", "yêu", "uêu",
];

/// What a key does to a syllable, before it is checked. The keys with an
/// effect are also the non-letters a syllable takes (`Composer::is_word_key`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Effect {
    Tone(Tone),
    /// Remove the tone (Telex `z`, VNI and VIQR `0`).
    ClearTone,
    /// â, ê, ô: on the given base vowel (Telex `aa`), or on any of a, e, o
    /// (VNI `6`, VIQR `^`).
    Circumflex(Option<char>),
    /// ă (VNI `8`, VIQR `(`).
    Breve,
    /// ơ, ư (VNI `7`, VIQR `+`).
    Horn,
    /// Telex `w`: ơ, ư or ă, and, if `lone`, `ư` on its own.
    W {
        lone: bool,
    },
    /// Telex `[` and `]`: the letter ơ or ư, given by its base vowel.
    Horned(char),
    /// đ.
    Stroke,
    /// VIQR `\`: the next mark key is typed as itself (RFC 1456), as in
    /// `kho^ng\?` for `không?`.
    Escape,
}

impl Effect {
    pub(super) fn of(options: &VietnameseOptions, key: char) -> Option<Self> {
        use InputMethod::{Telex, Viqr, Vni};
        let effect = match (options.method, key.to_ascii_lowercase()) {
            (Telex, 's') | (Vni, '1') | (Viqr, '\'') => Self::Tone(Tone::Acute),
            (Telex, 'f') | (Vni, '2') | (Viqr, '`') => Self::Tone(Tone::Grave),
            (Telex, 'r') | (Vni, '3') | (Viqr, '?') => Self::Tone(Tone::Hook),
            (Telex, 'x') | (Vni, '4') | (Viqr, '~') => Self::Tone(Tone::Tilde),
            (Telex, 'j') | (Vni, '5') | (Viqr, '.') => Self::Tone(Tone::Dot),
            (Telex, 'z') | (Vni | Viqr, '0') => Self::ClearTone,
            (Telex, c @ ('a' | 'e' | 'o')) => Self::Circumflex(Some(c)),
            (Vni, '6') | (Viqr, '^') => Self::Circumflex(None),
            (Vni, '7') | (Viqr, '+') => Self::Horn,
            (Vni, '8') | (Viqr, '(') => Self::Breve,
            (Telex, 'w') => Self::W {
                lone: options.telex_w,
            },
            (Telex, '[') if options.telex_brackets => Self::Horned('o'),
            (Telex, ']') if options.telex_brackets => Self::Horned('u'),
            (Telex | Viqr, 'd') | (Vni, '9') => Self::Stroke,
            (Viqr, '\\') => Self::Escape,
            _ => return None,
        };
        Some(effect)
    }
}

/// How a key's effect turned out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Applied,
    /// The key took its own mark off again (`ass` is `as`): the key is typed as
    /// a letter, and the syllable stops changing.
    Undone,
    /// The key took off a mark that stood for the key itself (`ww` is `w`):
    /// nothing more is typed, and the syllable stops changing.
    Reverted,
    /// VIQR: the key is punctuation after a syllable that already has its tone
    /// (`Vie^.t.` is `Việt.`): it is typed, and the syllable stops changing.
    Literal,
    /// VIQR `\`: it shows until the next key, which it may turn into itself.
    Escape,
    /// The key has nothing to change here and is typed as itself.
    NotApplicable,
}

/// The syllable being typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Word {
    letters: Vec<Letter>,
    tone: Tone,
    /// Set after an undo, so `ass` stays `as` and `class` stays `class`.
    frozen: bool,
    /// The keys typed into the syllable, to put it back as typed.
    keys: Vec<char>,
    /// A key that is not a letter (VIQR punctuation, a Telex bracket) marked
    /// the syllable. If a later letter makes it impossible in Vietnamese, it
    /// goes back to the keys as typed: `don't`, `ma.com`, `m[k]`.
    marked_by_punctuation: bool,
    /// VIQR: the key that changed the tone with the last key, and the tone
    /// before it, so the same key at once takes it back (`ba.n??` is `bạn?`).
    tone_by: Option<(char, Tone)>,
    /// VIQR: the last key was `\`.
    escaped: bool,
}

impl Word {
    pub fn is_empty(&self) -> bool {
        self.letters.is_empty()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The syllable as text, with the tone on the vowel `style` picks.
    pub fn render(&self, style: ToneStyle) -> String {
        let at = self.tone_position(style);
        self.letters
            .iter()
            .enumerate()
            .map(|(i, letter)| {
                let tone = if Some(i) == at {
                    self.tone
                } else {
                    Tone::Level
                };
                letter.render(tone)
            })
            .collect()
    }

    /// Removes the last character shown. If it carried the tone, the tone goes
    /// with it; otherwise the tone stays and may move (`hoàn` ⌫ is `hòa`).
    pub fn pop(&mut self, style: ToneStyle) {
        let at = self.tone_position(style);
        self.letters.pop();
        if at == Some(self.letters.len()) {
            self.tone = Tone::Level;
        }
        self.frozen = false;
        // The keys no longer match the letters: nothing to put back any more.
        self.keys.clear();
        self.marked_by_punctuation = false;
        self.tone_by = None;
        self.escaped = false;
        self.drop_stray_tone();
        if self.letters.is_empty() {
            self.clear();
        }
    }

    /// A tone with no vowel left to carry it (after a revert or Backspace)
    /// must not come back on the next vowel: `th[f[` ⌫ `a` is `tha`.
    fn drop_stray_tone(&mut self) {
        if self.vowels().is_none() {
            self.tone = Tone::Level;
        }
    }

    /// Adds a typed key: a mark, a tone or a letter. Returns false for a key
    /// that is not a letter and has nothing to change (a VNI digit, VIQR
    /// punctuation, a Telex bracket), which is then typed as itself outside
    /// the syllable.
    pub fn push_key(&mut self, key: char, options: &VietnameseOptions) -> bool {
        // Only the very next key can take back a VIQR tone.
        let tone_by = self.tone_by.take();
        if std::mem::take(&mut self.escaped) && (key == '\\' || Effect::of(options, key).is_some())
        {
            // `\?` is `?`: the key takes the place of the `\` shown.
            if self.letters.last().is_some_and(|l| l.base == '\\') {
                self.letters.pop();
            }
            return self.push_literal(key);
        }
        if !self.frozen
            && let Some(effect) = Effect::of(options, key)
        {
            let before = self.clone();
            let viqr = options.method == InputMethod::Viqr;
            match self.apply(effect, key, viqr, tone_by) {
                Outcome::Applied if options.check_spelling && !self.is_valid() => {
                    *self = before;
                }
                Outcome::Applied => {
                    if (viqr && !key.is_ascii_alphabetic()) || matches!(effect, Effect::Horned(_)) {
                        self.marked_by_punctuation = true;
                    }
                    self.keys.push(key);
                    return true;
                }
                Outcome::Undone | Outcome::Literal => return self.push_literal(key),
                Outcome::Reverted => {
                    self.frozen = true;
                    self.keys.push(key);
                    self.drop_stray_tone();
                    return true;
                }
                Outcome::Escape => {
                    self.letters.push(Letter::typed(key));
                    self.keys.push(key);
                    self.escaped = true;
                    return true;
                }
                Outcome::NotApplicable => {}
            }
        }
        if !key.is_ascii_alphabetic() {
            return false;
        }
        self.letters.push(Letter::typed(key));
        self.keys.push(key);
        if options.check_spelling && self.marked_by_punctuation && !self.is_valid() {
            // Not Vietnamese after all (`don't`, `ma.com`): back to the keys.
            self.letters = self.keys.iter().map(|&k| Letter::typed(k)).collect();
            self.tone = Tone::Level;
            self.frozen = true;
            self.marked_by_punctuation = false;
        }
        true
    }

    /// Types `key` as itself; the syllable stops changing.
    fn push_literal(&mut self, key: char) -> bool {
        self.letters.push(Letter::typed(key));
        self.keys.push(key);
        self.frozen = true;
        true
    }

    /// `tone_by` is the key that changed the tone with the previous key, and
    /// the tone before it.
    fn apply(
        &mut self,
        effect: Effect,
        key: char,
        viqr: bool,
        tone_by: Option<(char, Tone)>,
    ) -> Outcome {
        let vowels = self.vowels();
        match effect {
            Effect::Tone(_) if vowels.is_none() => Outcome::NotApplicable,
            // VIQR tone keys are punctuation too. The same key again at once
            // takes its tone back and is typed (`ba.n??` is `bạn?`); a tone the
            // syllable already has makes the key punctuation (`Vie^.t.` is
            // `Việt.`).
            Effect::Tone(_) if viqr && tone_by.is_some_and(|(by, _)| by == key) => {
                self.tone = tone_by.map_or(Tone::Level, |(_, previous)| previous);
                Outcome::Undone
            }
            Effect::Tone(tone) if viqr && self.tone == tone => Outcome::Literal,
            Effect::Tone(tone) if viqr => {
                self.tone_by = Some((key, self.tone));
                self.tone = tone;
                Outcome::Applied
            }
            Effect::Tone(tone) => {
                if self.tone == tone {
                    self.tone = Tone::Level;
                    Outcome::Undone
                } else {
                    self.tone = tone;
                    Outcome::Applied
                }
            }
            Effect::ClearTone if self.tone != Tone::Level => {
                self.tone = Tone::Level;
                Outcome::Applied
            }
            Effect::ClearTone => Outcome::NotApplicable,
            Effect::Circumflex(base) => {
                let Some(vowels) = vowels else {
                    return Outcome::NotApplicable;
                };
                let fits = |l: &Letter| match base {
                    Some(base) => l.base == base,
                    None => matches!(l.base, 'a' | 'e' | 'o'),
                };
                self.toggle_mark(vowels, Mark::Circumflex, fits)
            }
            Effect::Breve => {
                let Some(vowels) = vowels else {
                    return Outcome::NotApplicable;
                };
                self.toggle_mark(vowels, Mark::Breve, |l| l.base == 'a')
            }
            Effect::Horn | Effect::W { .. } => match vowels {
                Some(vowels) => self.horn(vowels, matches!(effect, Effect::W { .. })),
                None if effect == (Effect::W { lone: true }) => {
                    // A lone Telex `w` is `ư`: `w` is "ư", `nhw` is "như".
                    self.letters.push(Letter {
                        mark: Mark::Horn,
                        made_by: Some('w'),
                        ..Letter::typed(if key.is_uppercase() { 'U' } else { 'u' })
                    });
                    Outcome::Applied
                }
                None => Outcome::NotApplicable,
            },
            Effect::Horned(base) => match self.letters.last_mut() {
                // `[[` is `[`: the letter the bracket made turns back into it.
                Some(last) if last.made_by == Some(key) => {
                    *last = Letter::typed(key);
                    Outcome::Reverted
                }
                _ => {
                    // Brackets have no capitals: an all-capital word gets one
                    // (`NG][IF` is `NGƯỜI`), a single capital does not (`T[` is `Tơ`).
                    let upper = self.letters.len() >= 2 && self.letters.iter().all(|l| l.upper);
                    self.letters.push(Letter {
                        mark: Mark::Horn,
                        made_by: Some(key),
                        ..Letter::typed(if upper {
                            base.to_ascii_uppercase()
                        } else {
                            base
                        })
                    });
                    Outcome::Applied
                }
            },
            Effect::Escape => Outcome::Escape,
            Effect::Stroke => match self.letters.first_mut() {
                Some(first) if first.base == 'd' && first.mark == Mark::Stroke => {
                    first.mark = Mark::None;
                    Outcome::Undone
                }
                Some(first) if first.base == 'd' => {
                    first.mark = Mark::Stroke;
                    Outcome::Applied
                }
                _ => Outcome::NotApplicable,
            },
        }
    }

    /// Puts `mark` on the last vowel that `fits`, or takes it off if a vowel
    /// already has it.
    fn toggle_mark(
        &mut self,
        vowels: Range<usize>,
        mark: Mark,
        fits: impl Fn(&Letter) -> bool,
    ) -> Outcome {
        let group = &mut self.letters[vowels];
        if group.iter().any(|l| fits(l) && l.mark == mark) {
            for letter in group.iter_mut().filter(|l| fits(l)) {
                letter.mark = Mark::None;
            }
            return Outcome::Undone;
        }
        match group.iter_mut().rev().find(|l| fits(l)) {
            Some(letter) => {
                letter.mark = mark;
                Outcome::Applied
            }
            None => Outcome::NotApplicable,
        }
    }

    /// The horn (and, for Telex `w`, the breve): `uo` is `ươ`, `ua` is `ưa`,
    /// then `ă`, `ơ` or `ư`. A second `w`, `7` or `+` takes them off.
    fn horn(&mut self, vowels: Range<usize>, w_key: bool) -> Outcome {
        let has_final = vowels.end < self.letters.len();
        let horned =
            |l: &Letter| l.mark == Mark::Horn || (w_key && l.mark == Mark::Breve && l.base == 'a');
        // `ưo` and `uơ` become `ươ` (`nguwow`), before a second key undoes.
        let pair = self.letters[vowels.clone()]
            .windows(2)
            .position(|w| w[0].base == 'u' && w[1].base == 'o')
            .map(|i| vowels.start + i);
        if let Some(i) = pair
            && (self.letters[i].mark != Mark::Horn || self.letters[i + 1].mark != Mark::Horn)
        {
            self.letters[i].mark = Mark::Horn;
            self.letters[i + 1].mark = Mark::Horn;
            return Outcome::Applied;
        }
        if self.letters[vowels.clone()].iter().any(horned) {
            // `ww` is `w`: the `ư` a lone `w` made turns back into the `w`.
            if let Some(made) = self
                .letters
                .iter_mut()
                .find(|l| w_key && l.made_by == Some('w'))
            {
                *made = Letter::typed(if made.upper { 'W' } else { 'w' });
                return Outcome::Reverted;
            }
            for letter in &mut self.letters[vowels] {
                if horned(letter) {
                    letter.mark = Mark::None;
                }
            }
            return Outcome::Undone;
        }

        let bases: Vec<char> = self.letters[vowels.clone()]
            .iter()
            .map(|l| l.base)
            .collect();
        let start = vowels.start;
        let mut set = |i: usize, mark: Mark| self.letters[start + i].mark = mark;
        if bases == ['u', 'a'] && !has_final {
            set(0, Mark::Horn);
        } else if let Some(i) = bases.iter().rposition(|&b| b == 'a').filter(|_| w_key) {
            set(i, Mark::Breve);
        } else if let Some(i) = bases.iter().rposition(|&b| b == 'o') {
            set(i, Mark::Horn);
        } else if let Some(i) = bases.iter().position(|&b| b == 'u') {
            set(i, Mark::Horn);
        } else {
            return Outcome::NotApplicable;
        }
        Outcome::Applied
    }

    /// Where the vowel group is. The `u` of `qu` and the `i` of `gi` belong to
    /// the consonant when another vowel follows.
    fn vowels(&self) -> Option<Range<usize>> {
        let letters = &self.letters;
        let mut start = letters.iter().position(Letter::is_vowel)?;
        let glide = |first: char, second: char| {
            start == 1
                && letters[0].base == first
                && letters[1].base == second
                && letters[1].mark == Mark::None
                && letters.get(2).is_some_and(Letter::is_vowel)
        };
        if glide('q', 'u') || glide('g', 'i') {
            start = 2;
        }
        let len = letters[start..].iter().take_while(|l| l.is_vowel()).count();
        Some(start..start + len)
    }

    /// The letter that carries the tone.
    fn tone_position(&self, style: ToneStyle) -> Option<usize> {
        let vowels = self.vowels()?;
        let group = &self.letters[vowels.clone()];
        // A vowel with a shape mark takes the tone; in ươ, the ơ.
        if let Some(i) = group.iter().rposition(|l| l.mark != Mark::None) {
            return Some(vowels.start + i);
        }
        // Punctuation typed into the syllable (`Kho?e?`) is not a final.
        let has_final = self.letters[vowels.end..]
            .iter()
            .any(|l| l.base.is_ascii_alphabetic());
        let offset = match group.len() {
            1 => 0,
            _ if has_final => group.len() - 1,
            2 => {
                let pair = [group[0].base, group[1].base];
                let modern = matches!(pair, ['o', 'a'] | ['o', 'e'] | ['u', 'y']);
                usize::from(style == ToneStyle::Modern && modern)
            }
            _ => 1,
        };
        Some(vowels.start + offset)
    }

    /// Whether the syllable could be Vietnamese: a known initial consonant,
    /// vowel group and final consonant, and a tone its final allows.
    fn is_valid(&self) -> bool {
        let spell = |letters: &[Letter]| -> String { letters.iter().map(|l| l.base).collect() };
        let stroke_ok = self
            .letters
            .iter()
            .enumerate()
            .all(|(i, l)| l.mark != Mark::Stroke || i == 0);
        let Some(vowels) = self.vowels() else {
            return stroke_ok && INITIALS.contains(&spell(&self.letters).as_str());
        };
        let initial = spell(&self.letters[..vowels.start]);
        let last = spell(&self.letters[vowels.end..]);
        let stop = STOP_FINALS.contains(&last.as_str());
        stroke_ok
            && INITIALS.contains(&initial.as_str())
            && FINALS.contains(&last.as_str())
            && is_vowel_group(&self.letters[vowels])
            && (!stop || matches!(self.tone, Tone::Level | Tone::Acute | Tone::Dot))
    }
}

fn is_vowel_group(group: &[Letter]) -> bool {
    VOWEL_GROUPS.iter().any(|known| {
        known.chars().count() == group.len()
            && known.chars().zip(group).all(|(c, letter)| {
                let (base, mark) = vowel_parts(c).unwrap_or((c, Mark::None));
                letter.base == base && (letter.mark == Mark::None || letter.mark == mark)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(keys: &str, options: &VietnameseOptions) -> String {
        let mut word = Word::default();
        for key in keys.chars() {
            assert!(word.push_key(key, options), "{key} in {keys}");
        }
        word.render(options.tone_style)
    }

    #[test]
    fn every_listed_vowel_group_is_valid() {
        for group in VOWEL_GROUPS {
            let letters: Vec<Letter> = group
                .chars()
                .map(|c| {
                    let (base, mark) = vowel_parts(c).unwrap_or((c, Mark::None));
                    Letter {
                        mark,
                        ..Letter::typed(base)
                    }
                })
                .collect();
            assert!(is_vowel_group(&letters), "{group}");
        }
    }

    #[test]
    fn the_tone_moves_as_the_syllable_grows() {
        let options = VietnameseOptions::default();
        let mut word = Word::default();
        for key in "hoaf".chars() {
            word.push_key(key, &options);
        }
        assert_eq!(word.render(ToneStyle::Classic), "hòa");
        assert_eq!(word.render(ToneStyle::Modern), "hoà");
        word.push_key('n', &options);
        assert_eq!(word.render(ToneStyle::Classic), "hoàn");
        word.pop(ToneStyle::Classic);
        assert_eq!(word.render(ToneStyle::Classic), "hòa");
        word.pop(ToneStyle::Classic);
        assert_eq!(
            word.render(ToneStyle::Classic),
            "hò",
            "the tone is on the o"
        );
        word.pop(ToneStyle::Classic);
        assert_eq!(
            word.render(ToneStyle::Classic),
            "h",
            "the tone left with the o"
        );
        word.push_key('a', &options);
        assert_eq!(word.render(ToneStyle::Classic), "ha");
    }

    #[test]
    fn invalid_syllables_are_left_alone_only_when_checking() {
        let check = VietnameseOptions::default();
        let loose = VietnameseOptions {
            check_spelling: false,
            ..check
        };
        assert_eq!(typed("for", &check), "for");
        assert_eq!(typed("for", &loose), "fỏ");
        assert_eq!(
            typed("hocf", &check),
            "hocf",
            "stop finals take sắc or nặng"
        );
        assert_eq!(typed("hocj", &check), "học");
    }
}
