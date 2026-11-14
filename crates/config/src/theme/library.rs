//! Built-in and user themes, by name.

use std::path::PathBuf;

use super::{Appearance, Theme, builtin_themes};

/// Where a theme came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeOrigin {
    Builtin,
    File(PathBuf),
}

/// A theme that can be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    /// How settings refer to it: the file name without `.json`.
    pub id: String,
    pub theme: Theme,
    pub origin: ThemeOrigin,
}

impl ThemeEntry {
    pub fn is_builtin(&self) -> bool {
        self.origin == ThemeOrigin::Builtin
    }
}

/// Every theme that can be chosen: the built-in ones, then user themes sorted
/// by name. A user theme named like a built-in one replaces it, so copying
/// `nord.json` into `themes/` is how to change the Nord theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeLibrary {
    entries: Vec<ThemeEntry>,
    /// The first `builtin_slots` entries are the built-in themes, or user
    /// themes that replaced them.
    builtin_slots: usize,
}

impl ThemeLibrary {
    /// Only the built-in themes.
    pub fn builtin() -> Self {
        let entries: Vec<ThemeEntry> = builtin_themes()
            .iter()
            .map(|(id, theme)| ThemeEntry {
                id: (*id).to_owned(),
                theme: theme.clone(),
                origin: ThemeOrigin::Builtin,
            })
            .collect();
        Self {
            builtin_slots: entries.len(),
            entries,
        }
    }

    pub fn entries(&self) -> &[ThemeEntry] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&ThemeEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// The theme `id`, or the default theme of `appearance` if there is none.
    pub fn get_or_default(&self, id: &str, appearance: Appearance) -> &ThemeEntry {
        self.get(id)
            .or_else(|| self.get(appearance.default_theme()))
            .expect("the built-in default themes are always present")
    }

    /// Adds a theme, replacing any theme with the same name.
    pub fn insert(&mut self, entry: ThemeEntry) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry;
            return;
        }
        self.entries.push(entry);
        self.entries[self.builtin_slots..].sort_by(|a, b| a.id.cmp(&b.id));
    }
}

impl Default for ThemeLibrary {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: &str, title: &str) -> ThemeEntry {
        let source = format!(r#"{{ "title": "{title}", "appearance": "light" }}"#);
        ThemeEntry {
            id: id.to_owned(),
            theme: Theme::from_json(&source).unwrap(),
            origin: ThemeOrigin::File(PathBuf::from(format!("{id}.json"))),
        }
    }

    #[test]
    fn builtins_come_first_then_user_themes_by_name() {
        let mut library = ThemeLibrary::builtin();
        let builtins = library.entries().len();
        for id in ["zebra", "apple"] {
            library.insert(user(id, id));
        }
        let ids: Vec<&str> = library.entries()[builtins..]
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(ids, ["apple", "zebra"]);
        assert!(library.get("dark").unwrap().is_builtin());
    }

    #[test]
    fn a_user_theme_replaces_a_builtin_with_the_same_name() {
        let mut library = ThemeLibrary::builtin();
        let count = library.entries().len();
        library.insert(user("nord", "My Nord"));
        assert_eq!(library.entries().len(), count);
        let nord = library.get("nord").unwrap();
        assert_eq!(nord.theme.title, "My Nord");
        assert!(!nord.is_builtin());
    }

    #[test]
    fn missing_themes_fall_back_to_the_default_of_their_appearance() {
        let library = ThemeLibrary::builtin();
        assert_eq!(
            library.get_or_default("nope", Appearance::Light).id,
            "light"
        );
        assert_eq!(library.get_or_default("nope", Appearance::Dark).id, "dark");
        assert_eq!(library.get_or_default("nord", Appearance::Light).id, "nord");
    }
}
