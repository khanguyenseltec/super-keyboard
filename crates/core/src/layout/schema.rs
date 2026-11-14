//! The shape of a layout file, as written in TOML.
//!
//! These types only describe the file; [`super::resolve`] checks them and turns
//! them into a [`super::Layout`].

use std::fmt;

use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, value::MapAccessDeserializer},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LayoutFile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sections: Vec<SectionFile>,
    #[serde(default)]
    pub layers: Vec<LayerFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SectionFile {
    pub name: String,
    pub title: Option<String>,
    pub enabled: Option<bool>,
    pub required: Option<bool>,
    pub rows: Vec<Vec<KeyFile>>,
}

/// A layer lists either its `sections` or, for simple layouts, its `rows`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LayerFile {
    pub name: String,
    pub rows: Option<Vec<Vec<KeyFile>>>,
    pub sections: Option<Vec<String>>,
}

/// A key: either the short notation (`"<esc>"`) or a table with options.
#[derive(Debug)]
pub(super) enum KeyFile {
    Short(String),
    Table(KeyTable),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KeyTable {
    /// Key notation, as in the short form.
    pub tap: Option<String>,
    /// Literal text, never parsed as key notation.
    pub text: Option<String>,
    /// Key notation for a long press.
    pub hold: Option<String>,
    pub label: Option<String>,
    pub width: Option<f32>,
    pub repeat: Option<bool>,
    pub swipe: Option<String>,
}

// Written by hand rather than `#[serde(untagged)]` so that mistakes inside a key
// table (`{ hodl = "x" }`) report the real problem instead of "did not match any
// variant".
impl<'de> Deserialize<'de> for KeyFile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;

        impl<'de> de::Visitor<'de> for KeyVisitor {
            type Value = KeyFile;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(r#"a key such as "a" or "<esc>", or a table such as { tap = "<space>", width = 3.0 }"#)
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<KeyFile, E> {
                Ok(KeyFile::Short(value.to_owned()))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<KeyFile, A::Error> {
                KeyTable::deserialize(MapAccessDeserializer::new(map)).map(KeyFile::Table)
            }
        }

        deserializer.deserialize_any(KeyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mistakes_in_key_tables_name_the_field() {
        let err = toml::from_str::<LayoutFile>(
            r#"
            name = "t"
            [[layers]]
            name = "a"
            rows = [[{ tap = "x", hodl = "y" }]]
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown field `hodl`"), "{err}");
    }

    #[test]
    fn keys_must_be_strings_or_tables() {
        let err = toml::from_str::<LayoutFile>(
            r#"
            name = "t"
            [[layers]]
            name = "a"
            rows = [[42]]
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("a key such as"), "{err}");
    }
}
