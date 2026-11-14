//! Keyboard layouts: layers of rows of keys, loaded from TOML.
//!
//! ```toml
//! name = "tiny"
//!
//! [[layers]]
//! name = "base"
//! rows = [
//!     ["<esc>", "<ctrl>", "->", "<layer:nav>"],
//!     ["a", "b", { tap = "<space>", width = 3.0 }, { text = "<div>", label = "div" }],
//! ]
//!
//! [[layers]]
//! name = "nav"
//! rows = [["<left>", "<down>", "<up>", "<right>", "<layer:nav>"]]
//! ```
//!
//! The first layer is the one shown when the keyboard starts. See [`crate::action`]
//! for the key notation.

use std::collections::HashSet;

use serde::Deserialize;

use crate::action::Action;

/// The developer layout that ships with Super Keyboard.
pub const DEV_LAYOUT_TOML: &str = include_str!("../layouts/dev.toml");

/// A complete keyboard layout.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub name: String,
    pub description: String,
    /// Never empty; the first layer is the base layer.
    pub layers: Vec<Layer>,
}

/// One layer of a layout, such as letters, symbols or navigation.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub rows: Vec<Vec<Key>>,
}

/// One key on a layer.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub label: String,
    pub action: Action,
    /// Relative width; a normal key is `1.0`.
    pub width: f32,
}

/// Why a layout file could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("invalid layout file: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("the layout has no layers")]
    NoLayers,
    #[error("layer `{0}` is defined more than once")]
    DuplicateLayer(String),
    #[error("layer `{0}` has no keys")]
    EmptyLayer(String),
    #[error("layer `{layer}`, row {row}, key {key}: {reason}")]
    InvalidKey {
        layer: String,
        /// 1-based.
        row: usize,
        /// 1-based.
        key: usize,
        reason: String,
    },
}

impl Layout {
    /// The developer layout that ships with Super Keyboard.
    pub fn builtin() -> Self {
        Self::from_toml(DEV_LAYOUT_TOML).expect("the built-in layout is valid")
    }

    /// Parses and validates a layout file.
    pub fn from_toml(source: &str) -> Result<Self, LayoutError> {
        let file: LayoutFile = toml::from_str(source)?;
        file.validate()
    }

    /// Index of the layer with this name.
    pub fn layer_index(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|layer| layer.name == name)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutFile {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    layers: Vec<LayerFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayerFile {
    name: String,
    rows: Vec<Vec<KeyFile>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum KeyFile {
    Short(String),
    Full(FullKey),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FullKey {
    /// Key notation, as in the short form.
    tap: Option<String>,
    /// Literal text, never parsed as key notation.
    text: Option<String>,
    label: Option<String>,
    width: Option<f32>,
}

impl LayoutFile {
    fn validate(self) -> Result<Layout, LayoutError> {
        if self.layers.is_empty() {
            return Err(LayoutError::NoLayers);
        }
        let names: HashSet<&str> = self.layers.iter().map(|l| l.name.as_str()).collect();
        if names.len() != self.layers.len() {
            let mut seen = HashSet::new();
            let duplicate = self.layers.iter().find(|l| !seen.insert(&l.name));
            let name = duplicate.map(|l| l.name.clone()).unwrap_or_default();
            return Err(LayoutError::DuplicateLayer(name));
        }

        let mut layers = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            if layer.rows.iter().all(Vec::is_empty) {
                return Err(LayoutError::EmptyLayer(layer.name.clone()));
            }
            let mut rows = Vec::with_capacity(layer.rows.len());
            for (r, row) in layer.rows.iter().enumerate() {
                let mut keys = Vec::with_capacity(row.len());
                for (k, key) in row.iter().enumerate() {
                    let key = key
                        .resolve(&names)
                        .map_err(|reason| LayoutError::InvalidKey {
                            layer: layer.name.clone(),
                            row: r + 1,
                            key: k + 1,
                            reason,
                        })?;
                    keys.push(key);
                }
                rows.push(keys);
            }
            layers.push(Layer {
                name: layer.name.clone(),
                rows,
            });
        }

        Ok(Layout {
            name: self.name,
            description: self.description,
            layers,
        })
    }
}

impl KeyFile {
    fn resolve(&self, layers: &HashSet<&str>) -> Result<Key, String> {
        let (action, label, width) = match self {
            Self::Short(spec) => (Action::parse(spec)?, None, None),
            Self::Full(full) => {
                let action = match (&full.tap, &full.text) {
                    (Some(spec), None) => Action::parse(spec)?,
                    (None, Some(text)) if !text.is_empty() => Action::Text(text.clone()),
                    (None, Some(_)) => return Err("`text` is empty".to_owned()),
                    _ => return Err("set exactly one of `tap` or `text`".to_owned()),
                };
                (action, full.label.clone(), full.width)
            }
        };
        if let Action::Layer(target) = &action
            && !layers.contains(target.as_str())
        {
            return Err(format!("switches to unknown layer `{target}`"));
        }
        let width = width.unwrap_or(1.0);
        if !(width.is_finite() && width > 0.0) {
            return Err(format!("width must be a positive number, not {width}"));
        }
        Ok(Key {
            label: label.unwrap_or_else(|| action.default_label()),
            action,
            width,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Modifier, NamedKey};

    #[test]
    fn builtin_layout_is_valid() {
        let layout = Layout::builtin();
        assert_eq!(layout.layers[0].name, "base");
        assert!(layout.layer_index("sym").is_some());
        assert!(layout.layer_index("nav").is_some());
    }

    #[test]
    fn parses_short_and_table_keys() {
        let layout = Layout::from_toml(
            r#"
            name = "t"
            [[layers]]
            name = "base"
            rows = [["<ctrl>", { tap = "<space>", width = 3.0 }, { text = "<div>", label = "div" }]]
            "#,
        )
        .unwrap();
        let row = &layout.layers[0].rows[0];
        assert_eq!(row[0].action, Action::Modifier(Modifier::Ctrl));
        assert_eq!(row[0].label, "ctrl");
        assert_eq!(row[1].action, Action::Key(NamedKey::Space));
        assert_eq!(row[1].width, 3.0);
        assert_eq!(row[2].action, Action::Text("<div>".to_owned()));
        assert_eq!(row[2].label, "div");
    }

    #[test]
    fn rejects_bad_layouts() {
        let bad = [
            r#"name = "t""#,
            r#"name = "t"
               [[layers]]
               name = "a"
               rows = [["x"]]
               [[layers]]
               name = "a"
               rows = [["y"]]"#,
            r#"name = "t"
               [[layers]]
               name = "a"
               rows = [["<layer:missing>"]]"#,
            r#"name = "t"
               [[layers]]
               name = "a"
               rows = [["<nope>"]]"#,
            r#"name = "t"
               [[layers]]
               name = "a"
               rows = [[{ tap = "x", width = 0.0 }]]"#,
            r#"name = "t"
               [[layers]]
               name = "a"
               rows = [[]]"#,
        ];
        for source in bad {
            assert!(Layout::from_toml(source).is_err(), "accepted:\n{source}");
        }
    }

    #[test]
    fn errors_point_at_the_key() {
        let err = Layout::from_toml(
            r#"name = "t"
               [[layers]]
               name = "base"
               rows = [["a", "b"], ["c", "<nope>"]]"#,
        )
        .unwrap_err();
        assert!(
            err.to_string().starts_with("layer `base`, row 2, key 2:"),
            "{err}"
        );
    }
}
