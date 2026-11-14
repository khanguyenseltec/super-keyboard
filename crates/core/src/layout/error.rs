//! Errors from loading a layout, precise enough to point at the faulty key.

use std::fmt;

use crate::action::ParseActionError;

/// Why a layout file could not be loaded.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LayoutError {
    /// The file is not valid TOML or does not have the layout's shape. The message
    /// includes the line and column.
    #[error("invalid layout file: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("the layout has no layers")]
    NoLayers,
    #[error("layer `{0}` is defined more than once")]
    DuplicateLayer(String),
    #[error("layer `{0}` has no keys")]
    EmptyLayer(String),
    #[error("layer `{0}` needs either `sections` or `rows`, not both")]
    LayerContent(String),
    #[error("section `{0}` is defined more than once, or has the name of a layer with `rows`")]
    DuplicateSection(String),
    #[error("section name `{0}` may only use letters, digits, `-` and `_`")]
    InvalidSectionName(String),
    #[error("section `{0}` has no keys")]
    EmptySection(String),
    #[error("section `{0}` is required, so it cannot be off by default")]
    RequiredSectionOff(String),
    #[error("layer `{layer}` uses unknown section `{section}`")]
    UnknownSection { layer: String, section: String },
    #[error(
        "layer `{layer}` lists section `{later}` after `{earlier}`, but `{later}` is declared \
         first; sections show in the order they are declared"
    )]
    SectionOrder {
        layer: String,
        earlier: String,
        later: String,
    },
    #[error("layer `{layer}` uses section `{section}` twice")]
    RepeatedSection { layer: String, section: String },
    #[error("{block}, row {row}, key {key}: {problem}")]
    InvalidKey {
        block: Block,
        /// 1-based.
        row: usize,
        /// 1-based.
        key: usize,
        problem: KeyProblem,
    },
}

/// Where a key is defined: in a section, or in a layer's own `rows`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Section(String),
    Layer(String),
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Section(name) => write!(f, "section `{name}`"),
            Self::Layer(name) => write!(f, "layer `{name}`"),
        }
    }
}

/// What is wrong with one key.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum KeyProblem {
    #[error("{0}{hint}", hint = text_form_hint(.0))]
    Tap(ParseActionError),
    #[error("`hold`: {0}")]
    Hold(ParseActionError),
    #[error("set exactly one of `tap` or `text`")]
    TapOrText,
    #[error("`text` is empty")]
    EmptyText,
    #[error("switches to unknown layer `{0}`")]
    UnknownLayer(String),
    #[error("width must be a positive number, not {0}")]
    InvalidWidth(f32),
    #[error("unknown swipe `{0}` (the only swipe is `cursor`)")]
    UnknownSwipe(String),
    #[error("a key that repeats cannot also have `{0}`")]
    RepeatConflict(&'static str),
}

/// Suggests the table form when a tap looks like a key name but is meant as text.
fn text_form_hint(error: &ParseActionError) -> String {
    match error {
        ParseActionError::UnknownName(name) => {
            format!(" (write {{ text = \"<{name}>\" }} to insert it as text)")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_errors_suggest_the_text_form() {
        let problem = KeyProblem::Tap(ParseActionError::UnknownName("div".into()));
        assert_eq!(
            problem.to_string(),
            r#"unknown key `<div>` (write { text = "<div>" } to insert it as text)"#
        );
        let problem = KeyProblem::Hold(ParseActionError::UnknownName("div".into()));
        assert_eq!(problem.to_string(), "`hold`: unknown key `<div>`");
    }
}
