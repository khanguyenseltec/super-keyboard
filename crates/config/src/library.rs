//! Built-in and user layouts, by id.

use std::{fmt, path::PathBuf};

use super_keyboard_core::Layout;

/// Names a layout: the file name without `.toml` for user layouts, the
/// layout's name for built-in ones. `settings.toml` refers to layouts by id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayoutId(String);

impl LayoutId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LayoutId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a layout came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutOrigin {
    Builtin,
    File(PathBuf),
}

/// A layout that can be chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutEntry {
    pub id: LayoutId,
    pub layout: Layout,
    pub origin: LayoutOrigin,
}

/// Every layout that can be chosen: the built-in ones, then user layouts sorted
/// by id. A user layout with a built-in's id replaces it, so copying `dev.toml`
/// into `layouts/` is how to customise the default layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutLibrary {
    entries: Vec<LayoutEntry>,
    /// The first `builtin_slots` entries are the built-in layouts, or user
    /// layouts that replaced them.
    builtin_slots: usize,
}

impl LayoutLibrary {
    /// The layout used when settings do not name one, or name a missing one.
    pub const DEFAULT: &str = "dev";

    /// Only the built-in layouts.
    pub fn builtin() -> Self {
        let layout = Layout::builtin();
        let entries = vec![LayoutEntry {
            id: LayoutId::new(layout.name.clone()),
            layout,
            origin: LayoutOrigin::Builtin,
        }];
        Self {
            builtin_slots: entries.len(),
            entries,
        }
    }

    pub fn entries(&self) -> &[LayoutEntry] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&LayoutEntry> {
        self.entries.iter().find(|entry| entry.id.as_str() == id)
    }

    /// The default layout: the user's `dev.toml` if there is one, else the
    /// built-in.
    pub fn default_entry(&self) -> &LayoutEntry {
        self.get(Self::DEFAULT)
            .expect("the built-in default layout is always present")
    }

    /// Adds a layout, replacing any layout with the same id.
    pub fn insert(&mut self, entry: LayoutEntry) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry;
            return;
        }
        self.entries.push(entry);
        self.entries[self.builtin_slots..].sort_by(|a, b| a.id.cmp(&b.id));
    }
}

impl Default for LayoutLibrary {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: &str) -> LayoutEntry {
        LayoutEntry {
            id: LayoutId::new(id),
            layout: Layout::builtin(),
            origin: LayoutOrigin::File(PathBuf::from(format!("{id}.toml"))),
        }
    }

    fn ids(library: &LayoutLibrary) -> Vec<&str> {
        library.entries().iter().map(|e| e.id.as_str()).collect()
    }

    #[test]
    fn builtins_come_first_then_user_layouts_by_id() {
        let mut library = LayoutLibrary::builtin();
        for id in ["vim", "abc", "mine"] {
            library.insert(user(id));
        }
        assert_eq!(ids(&library), ["dev", "abc", "mine", "vim"]);
        assert_eq!(library.default_entry().origin, LayoutOrigin::Builtin);
    }

    #[test]
    fn a_user_layout_replaces_a_builtin_with_the_same_id() {
        let mut library = LayoutLibrary::builtin();
        library.insert(user("dev"));
        assert_eq!(ids(&library), ["dev"]);
        assert!(matches!(
            library.default_entry().origin,
            LayoutOrigin::File(_)
        ));
    }
}
