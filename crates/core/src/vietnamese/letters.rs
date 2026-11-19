//! Vietnamese letters as parts: a base letter, a shape mark and a tone, and the
//! precomposed characters they make.

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

/// One letter of the syllable being typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Letter {
    /// The ASCII letter or digit the user typed, lowercase.
    pub base: char,
    pub upper: bool,
    pub mark: Mark,
    /// An `ư` made by a lone Telex `w`; typing `w` again turns it back into `w`.
    pub from_w: bool,
}

impl Letter {
    /// The letter for a typed key, keeping its case.
    pub fn typed(key: char) -> Self {
        Self {
            base: key.to_ascii_lowercase(),
            upper: key.is_uppercase(),
            mark: Mark::None,
            from_w: false,
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
            from_w: false,
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
    fn splits_toneless_vowels_into_parts() {
        assert_eq!(vowel_parts('ơ'), Some(('o', Mark::Horn)));
        assert_eq!(vowel_parts('a'), Some(('a', Mark::None)));
        assert_eq!(vowel_parts('á'), None);
        assert_eq!(vowel_parts('b'), None);
    }
}
