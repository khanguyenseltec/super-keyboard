//! The keys the keyboard runs: a layout with an arrangement applied.

use super::{Arrangement, Key, Layout};

/// A layout with its sections arranged: for each layer, the rows of the sections
/// that are on, in order. This is what [`crate::Keyboard`] runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Keymap {
    /// The layout's name.
    pub name: String,
    /// Never empty; the first layer is the base layer.
    pub layers: Vec<Layer>,
}

/// One layer as shown: rows of keys.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub rows: Vec<Vec<Key>>,
}

/// Where a key is in a [`Keymap`]: its layer, row and column (all 0-based).
///
/// Ids stay valid when the visible layer changes, so a press that started on one
/// layer still resolves to the same key after another finger switches layers.
/// Rearranging sections makes a new keymap, and with it new ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId {
    pub layer: usize,
    pub row: usize,
    pub col: usize,
}

impl Keymap {
    /// The built-in layout with its default sections.
    pub fn builtin() -> Self {
        Self::from(&Layout::builtin())
    }

    /// Index of the layer with this name.
    pub fn layer_index(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|layer| layer.name == name)
    }

    /// The key at `id`, if there is one.
    pub fn key(&self, id: KeyId) -> Option<&Key> {
        self.layers.get(id.layer)?.rows.get(id.row)?.get(id.col)
    }

    /// Every key of every layer, with its id.
    pub fn keys(&self) -> impl Iterator<Item = (KeyId, &Key)> {
        self.layers.iter().enumerate().flat_map(|(layer, l)| {
            l.rows.iter().enumerate().flat_map(move |(row, keys)| {
                keys.iter()
                    .enumerate()
                    .map(move |(col, key)| (KeyId { layer, row, col }, key))
            })
        })
    }
}

impl From<&Layout> for Keymap {
    /// The layout with every section in its default state and place.
    fn from(layout: &Layout) -> Self {
        layout.keymap(&Arrangement::default())
    }
}
