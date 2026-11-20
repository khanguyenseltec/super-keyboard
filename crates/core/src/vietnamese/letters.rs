//! Vietnamese letters as parts: a base letter, a shape mark and a tone, and the
//! characters they make, precomposed (NFC) or as a letter and its marks (NFD).

/// The six tones of a Vietnamese syllable. Only one vowel of a syllable carries
/// it; which one is decided by the whole syllable (see `Word::tone_position`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Tone {
    /// Thanh ngang: no mark.
    #[default]
    Level,
    /// Sắc: á.
    Acute,
    /// Huyền: à.
    Grave,
    /// Hỏi: ả.
    Hook,
    /// Ngã: ã.
    Tilde,
    /// Nặng: ạ.
    Dot,
}

impl Tone {
    fn index(self) -> usize {
        match self {
            Self::Level => 0,
            Self::Acute => 1,
            Self::Grave => 2,
            Self::Hook => 3,
            Self::Tilde => 4,
            Self::Dot => 5,
        }
    }
}

/// A mark that changes a letter's shape (and sound), as opposed to its tone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Mark {
    #[default]
    None,
    /// â, ê, ô.
    Circumflex,
    /// ă.
    Breve,
    /// ơ, ư.
    Horn,
    /// đ.
    Stroke,
}

/// The combining characters of the marks and tones, for decomposed output.
mod combining {
    pub const GRAVE: char = '\u{300}';
    pub const ACUTE: char = '\u{301}';
    pub const CIRCUMFLEX: char = '\u{302}';
    pub const TILDE: char = '\u{303}';
    pub const BREVE: char = '\u{306}';
    pub const HOOK_ABOVE: char = '\u{309}';
    pub const HORN: char = '\u{31b}';
    pub const DOT_BELOW: char = '\u{323}';
}

/// Every Vietnamese vowel with its shape mark, and its six tones in [`Tone`]
/// order.
const VOWELS: [(char, Mark, [char; 6]); 12] = [
    ('a', Mark::None, ['a', 'á', 'à', 'ả', 'ã', 'ạ']),
    ('a', Mark::Breve, ['ă', 'ắ', 'ằ', 'ẳ', 'ẵ', 'ặ']),
    ('a', Mark::Circumflex, ['â', 'ấ', 'ầ', 'ẩ', 'ẫ', 'ậ']),
    ('e', Mark::None, ['e', 'é', 'è', 'ẻ', 'ẽ', 'ẹ']),
    ('e', Mark::Circumflex, ['ê', 'ế', 'ề', 'ể', 'ễ', 'ệ']),
    ('i', Mark::None, ['i', 'í', 'ì', 'ỉ', 'ĩ', 'ị']),
    ('o', Mark::None, ['o', 'ó', 'ò', 'ỏ', 'õ', 'ọ']),
    ('o', Mark::Circumflex, ['ô', 'ố', 'ồ', 'ổ', 'ỗ', 'ộ']),
    ('o', Mark::Horn, ['ơ', 'ớ', 'ờ', 'ở', 'ỡ', 'ợ']),
    ('u', Mark::None, ['u', 'ú', 'ù', 'ủ', 'ũ', 'ụ']),
    ('u', Mark::Horn, ['ư', 'ứ', 'ừ', 'ử', 'ữ', 'ự']),
    ('y', Mark::None, ['y', 'ý', 'ỳ', 'ỷ', 'ỹ', 'ỵ']),
];

/// The base letter and shape mark of a toneless vowel such as `ơ`.
pub fn vowel_parts(c: char) -> Option<(char, Mark)> {
    VOWELS
        .iter()
        .find(|(_, _, forms)| forms[0] == c)
        .map(|&(base, mark, _)| (base, mark))
}

/// Appends `c` to `out` in Unicode's decomposed form (NFD): a marked vowel
/// becomes its base letter followed by combining marks. Anything else,
/// including `đ` (which Unicode does not decompose), is appended as it is.
///
/// The marks come in Unicode's canonical order: the horn, which is attached to
/// the letter, then the dot below, then the marks above (circumflex or breve,
/// and after it the tone).
pub fn decompose(c: char, out: &mut String) {
    let lower = c.to_lowercase().next().unwrap_or(c);
    let found = VOWELS.iter().find_map(|&(base, mark, forms)| {
        let tone = forms.iter().position(|&form| form == lower)?;
        Some((base, mark, tone))
    });
    let Some((base, mark, tone)) = found else {
        out.push(c);
        return;
    };
    out.push(if c == lower {
        base
    } else {
        base.to_ascii_uppercase()
    });
    let dot = tone == Tone::Dot.index();
    if mark == Mark::Horn {
        out.push(combining::HORN);
    }
    if dot {
        out.push(combining::DOT_BELOW);
    }
    match mark {
        Mark::Circumflex => out.push(combining::CIRCUMFLEX),
        Mark::Breve => out.push(combining::BREVE),
        Mark::None | Mark::Horn | Mark::Stroke => {}
    }
    let above = [
        None,
        Some(combining::ACUTE),
        Some(combining::GRAVE),
        Some(combining::HOOK_ABOVE),
        Some(combining::TILDE),
        None,
    ];
    out.extend(above[tone]);
}

/// One letter of the syllable being typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Letter {
    /// The ASCII letter or digit the user typed, lowercase.
    pub base: char,
    pub upper: bool,
    pub mark: Mark,
    /// The key that typed this letter whole: `w` for the `ư` of a lone Telex
    /// `w`, `[` or `]` for the `ơ` or `ư` of a Telex bracket. Typing the key
    /// again turns the letter back into the key.
    pub made_by: Option<char>,
}

impl Letter {
    /// The letter for a typed key, keeping its case.
    pub fn typed(key: char) -> Self {
        Self {
            base: key.to_ascii_lowercase(),
            upper: key.is_uppercase(),
            mark: Mark::None,
            made_by: None,
        }
    }

    pub fn is_vowel(&self) -> bool {
        matches!(self.base, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
    }

    /// The character for this letter, carrying `tone` if it is a vowel.
    pub fn render(&self, tone: Tone) -> char {
        let c = if self.base == 'd' && self.mark == Mark::Stroke {
            'đ'
        } else {
            VOWELS
                .iter()
                .find(|(base, mark, _)| *base == self.base && *mark == self.mark)
                .map_or(self.base, |(_, _, forms)| forms[tone.index()])
        };
        if self.upper {
            c.to_uppercase().next().unwrap_or(c)
        } else {
            c
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letter(base: char, mark: Mark, upper: bool) -> Letter {
        Letter {
            base,
            upper,
            mark,
            made_by: None,
        }
    }

    #[test]
    fn renders_every_vowel_with_every_tone_in_both_cases() {
        for (base, mark, forms) in VOWELS {
            let tones = [
                Tone::Level,
                Tone::Acute,
                Tone::Grave,
                Tone::Hook,
                Tone::Tilde,
                Tone::Dot,
            ];
            for (tone, expected) in tones.into_iter().zip(forms) {
                assert_eq!(letter(base, mark, false).render(tone), expected);
                let upper = letter(base, mark, true).render(tone);
                assert_eq!(upper.to_lowercase().next(), Some(expected));
                assert_ne!(upper, expected, "{expected} has an uppercase form");
            }
        }
    }

    #[test]
    fn renders_consonants_and_d_with_stroke() {
        assert_eq!(letter('d', Mark::Stroke, false).render(Tone::Level), 'đ');
        assert_eq!(letter('d', Mark::Stroke, true).render(Tone::Acute), 'Đ');
        assert_eq!(Letter::typed('N').render(Tone::Dot), 'N');
        assert_eq!(Letter::typed('7').render(Tone::Level), '7');
    }

    #[test]
    fn decomposes_every_vowel_as_unicode_nfd_does() {
        use unicode_normalization::UnicodeNormalization;
        for (_, _, forms) in VOWELS {
            for lower in forms {
                for c in [lower, lower.to_uppercase().next().unwrap()] {
                    let mut ours = String::new();
                    decompose(c, &mut ours);
                    let nfd: String = c.nfd().collect();
                    assert_eq!(ours, nfd, "{c}");
                    assert_eq!(ours.nfc().collect::<String>(), c.to_string(), "{c}");
                }
            }
        }
    }

    #[test]
    fn decomposing_leaves_other_characters_alone() {
        // đ has no decomposition in Unicode: it is a letter of its own.
        for c in ['đ', 'Đ', 'n', 'N', '7', ' '] {
            let mut out = String::new();
            decompose(c, &mut out);
            assert_eq!(out, c.to_string());
        }
    }

    #[test]
    fn splits_toneless_vowels_into_parts() {
        assert_eq!(vowel_parts('ơ'), Some(('o', Mark::Horn)));
        assert_eq!(vowel_parts('a'), Some(('a', Mark::None)));
        assert_eq!(vowel_parts('á'), None);
        assert_eq!(vowel_parts('b'), None);
    }
}
