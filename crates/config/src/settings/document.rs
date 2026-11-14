//! Saving settings back to `settings.toml` without losing the user's comments.

use std::fmt;

use std::collections::BTreeMap;

use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, TableLike, Value};

use super::{Settings, SettingsError};
use crate::profile::ProfileSpec;

/// The file written the first time settings are saved: every option, explained.
pub const TEMPLATE: &str = r##"# Super Keyboard settings.
#
# The app saves changes made on its Settings screen to this file and keeps your
# comments. You can edit it by hand too: the app reloads it when it changes.
# Every option is described in docs/CUSTOMIZING.md.

# Profile to use: a preset ("coding", "coding-numbers", "terminal", "compact"),
# one of your [[profiles]], or the name of a layout file in layouts/ without .toml.
#
# A profile picks a layout and switches its sections (number row, brackets,
# terminal keys...) on or off and reorders them. The settings screen edits
# profiles and the profile menu, and saves them at the end of this file:
#
#   [menu]                                  the profile menu
#   order = ["terminal", "coding"]          shown first, in this order
#   shown = { compact = false }             hidden from the menu
#
#   [[profiles]]
#   name = "mine"                           letters, digits, - and _
#   title = "Mine"
#   layout = "dev"
#   order = ["letters", "brackets"]         these sections in this order
#   sections = { number-row = true }        switched on, or off with false
profile = "coding"

# Colour theme: "system", "light" or "dark".
theme = "system"

[keys]
# Milliseconds before a long-press fires a key's hold action (150-1000).
hold-delay = 300
# Milliseconds before Backspace, Delete and the arrows start repeating (150-1000).
repeat-delay = 400
# Repeats per second (5-60).
repeat-rate = 30
# Pixels of drag per cursor step when swiping on the space bar (4-40).
swipe-step = 12
# Key height: "auto" sizes keys to the screen, or a number of pixels (32-96).
height = "auto"

[preview]
# Show recent output beside the preview text.
show-log = true

# Colours and sizes for both themes, then for each theme. Values are CSS colours
# (#7aa2f7, rgb(...), oklch(...), colour names) and lengths (6px, 0.5rem, calc(...)).
# Variables: bg, panel, key, key-special, key-pressed, text, muted, accent,
# on-accent, selection, warning, radius, gap, key-font.
[style]
# radius = "8px"

[style.dark]
# accent = "#ff9e64"

[style.light]
# accent = "#c2410c"
"##;

/// An editable `settings.toml`.
///
/// Values already in the file are changed in place, keeping their comments;
/// values that are not in the file are only added when they differ from the
/// default, so hand-written files stay short. `[style]` is never changed: it is
/// edited by hand.
#[derive(Debug, Clone)]
pub struct SettingsDocument {
    doc: DocumentMut,
}

impl SettingsDocument {
    /// Opens the current contents of `settings.toml`, or the commented template
    /// when there is no file or it is blank.
    ///
    /// A file with errors is refused: saving over it would throw away whatever
    /// the user is in the middle of fixing.
    pub fn open(source: Option<&str>) -> Result<Self, SettingsError> {
        let source = source.filter(|s| !s.trim().is_empty()).unwrap_or(TEMPLATE);
        Settings::from_toml(source)?;
        Ok(Self {
            doc: source.parse()?,
        })
    }

    /// Writes `settings` (except style) into the document.
    pub fn apply(&mut self, settings: &Settings) {
        let defaults = Settings::default();
        let root = self.doc.as_table_mut();
        match &settings.profile {
            Some(profile) => {
                set(root, "profile", Scalar::Str(profile.clone()), false);
            }
            None => {
                root.remove("profile");
            }
        }
        let theme = settings.theme;
        set(
            root,
            "theme",
            Scalar::Str(theme.name().into()),
            theme == defaults.theme,
        );

        let (keys, default_keys) = (&settings.keys, &defaults.keys);
        edit_section(root, "keys", |table| {
            let numbers = [
                ("hold-delay", keys.hold_delay_ms, default_keys.hold_delay_ms),
                (
                    "repeat-delay",
                    keys.repeat_delay_ms,
                    default_keys.repeat_delay_ms,
                ),
                ("repeat-rate", keys.repeat_rate, default_keys.repeat_rate),
                ("swipe-step", keys.swipe_step_px, default_keys.swipe_step_px),
            ];
            let mut added = false;
            for (key, value, default) in numbers {
                added |= set(table, key, Scalar::Int(value.into()), value == default);
            }
            let height = keys
                .height_px
                .map_or(Scalar::Str("auto".into()), |px| Scalar::Int(px.into()));
            added | set(table, "height", height, keys.height_px.is_none())
        });

        let show_log = settings.preview.show_log;
        edit_section(root, "preview", |table| {
            let is_default = show_log == defaults.preview.show_log;
            set(table, "show-log", Scalar::Bool(show_log), is_default)
        });

        let menu = &settings.menu;
        if !menu.is_default() || root.contains_key("menu") {
            edit_section(root, "menu", |table| {
                let order = Scalar::List(menu.order.clone());
                let shown = Scalar::Flags(menu.enabled.clone());
                set_or_remove(table, "order", order, menu.order.is_empty())
                    | set_or_remove(table, "shown", shown, menu.enabled.is_empty())
            });
        }

        write_profiles(root, &settings.profiles);
    }
}

/// Brings `[[profiles]]` in line with `profiles`: existing tables are updated in
/// place (keeping comments), new ones are added at the end, and tables for
/// profiles that are gone are removed.
fn write_profiles(root: &mut Table, profiles: &[ProfileSpec]) {
    if profiles.is_empty() && !root.contains_key("profiles") {
        return;
    }
    let item = root
        .entry("profiles")
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
    if !item.is_array_of_tables() {
        // `profiles = [{ ... }]` written inline becomes `[[profiles]]` tables.
        let converted = std::mem::take(item)
            .into_array_of_tables()
            .unwrap_or_default();
        *item = Item::ArrayOfTables(converted);
        if let Some(mut key) = root.key_mut("profiles") {
            key.fmt();
        }
    }
    let item = root.get_mut("profiles").expect("just inserted");
    let tables = item
        .as_array_of_tables_mut()
        .expect("just made an array of tables");
    let name_of = |table: &Table| table.get("name").and_then(Item::as_str).map(str::to_owned);
    tables
        .retain(|table| name_of(table).is_some_and(|name| profiles.iter().any(|p| p.name == name)));
    for spec in profiles {
        let index = tables
            .iter()
            .position(|table| name_of(table).as_deref() == Some(spec.name.as_str()));
        let table = match index {
            Some(index) => tables.get_mut(index).expect("index is in range"),
            None => {
                let mut table = Table::new();
                table.insert("name", Item::Value(spec.name.as_str().into()));
                tables.push(table);
                tables.get_mut(tables.len() - 1).expect("just pushed")
            }
        };
        write_profile(table, spec);
    }
    if tables.is_empty() {
        root.remove("profiles");
    }
}

fn write_profile(table: &mut Table, spec: &ProfileSpec) {
    let sections = &spec.sections;
    set_or_remove(
        table,
        "title",
        Scalar::Str(spec.title.clone()),
        spec.title == spec.name,
    );
    set(table, "layout", Scalar::Str(spec.layout.clone()), false);
    set_or_remove(
        table,
        "order",
        Scalar::List(sections.order.clone()),
        sections.order.is_empty(),
    );
    set_or_remove(
        table,
        "sections",
        Scalar::Flags(sections.enabled.clone()),
        sections.enabled.is_empty(),
    );
}

impl fmt::Display for SettingsDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.doc.fmt(f)
    }
}

/// The values settings are made of.
#[derive(Debug, Clone, PartialEq)]
enum Scalar {
    Int(i64),
    Bool(bool),
    Str(String),
    /// An array of strings, such as an order of names.
    List(Vec<String>),
    /// An inline table of switches, such as `{ number-row = true }`.
    Flags(BTreeMap<String, bool>),
}

impl Scalar {
    fn matches(&self, value: &Value) -> bool {
        match self {
            Self::Int(n) => value.as_integer() == Some(*n),
            Self::Bool(b) => value.as_bool() == Some(*b),
            Self::Str(s) => value.as_str() == Some(s.as_str()),
            Self::List(list) => value.as_array().is_some_and(|array| {
                array.len() == list.len()
                    && array.iter().zip(list).all(|(v, s)| v.as_str() == Some(s))
            }),
            Self::Flags(flags) => value.as_inline_table().is_some_and(|table| {
                table.len() == flags.len()
                    && flags
                        .iter()
                        .all(|(k, b)| table.get(k).and_then(Value::as_bool) == Some(*b))
            }),
        }
    }

    fn into_value(self) -> Value {
        match self {
            Self::Int(n) => n.into(),
            Self::Bool(b) => b.into(),
            Self::Str(s) => s.into(),
            Self::List(list) => Value::Array(list.into_iter().collect::<Array>()),
            Self::Flags(flags) => Value::InlineTable(flags.into_iter().collect::<InlineTable>()),
        }
    }
}

/// Sets `key` to `new`, keeping the comments around an existing value and leaving
/// an equal value untouched. A missing key is only added when `new` is not the
/// default. Returns whether a key was added.
fn set(table: &mut dyn TableLike, key: &str, new: Scalar, is_default: bool) -> bool {
    match table.get_mut(key) {
        Some(Item::Value(old)) => {
            if !new.matches(old) {
                let decor = old.decor().clone();
                *old = new.into_value();
                *old.decor_mut() = decor;
            }
            false
        }
        Some(item) => {
            *item = Item::Value(new.into_value());
            false
        }
        None if is_default => false,
        None => {
            table.insert(key, Item::Value(new.into_value()));
            true
        }
    }
}

/// Like [`set`], but removes the key when `new` is the default, for values that
/// only matter when they say something (a profile's order or switches).
fn set_or_remove(table: &mut dyn TableLike, key: &str, new: Scalar, is_default: bool) -> bool {
    if is_default {
        table.remove(key);
        false
    } else {
        set(table, key, new, false)
    }
}

/// Runs `edit` on the section `name` (a `[name]` table or an inline table),
/// creating it if missing; a section created for nothing is removed again. An
/// inline table that gained keys is reformatted, so `{ a = 1 }` grows into
/// `{ a = 1, b = 2 }`.
fn edit_section(root: &mut Table, name: &str, edit: impl FnOnce(&mut dyn TableLike) -> bool) {
    let existed = root.contains_key(name);
    let item = root
        .entry(name)
        .or_insert_with(|| Item::Table(Table::new()));
    if !item.is_table_like() {
        *item = Item::Table(Table::new());
    }
    let added = edit(item.as_table_like_mut().expect("just made table-like"));
    if added && let Some(inline) = item.as_inline_table_mut() {
        inline.fmt();
    }
    if !existed && item.as_table_like().is_some_and(|table| table.is_empty()) {
        root.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeySettings, ThemeMode};

    fn saved(source: Option<&str>, change: impl FnOnce(&mut Settings)) -> String {
        let mut settings = source.map_or_else(
            || Settings::from_toml(TEMPLATE).unwrap(),
            |s| Settings::from_toml(s).unwrap(),
        );
        change(&mut settings);
        let mut doc = SettingsDocument::open(source).unwrap();
        doc.apply(&settings);
        let text = doc.to_string();
        assert_eq!(Settings::from_toml(&text).unwrap(), settings, "{text}");
        text
    }

    #[test]
    fn the_template_describes_the_defaults() {
        let settings = Settings::from_toml(TEMPLATE).unwrap();
        assert_eq!(
            settings,
            Settings {
                profile: Some("coding".into()),
                ..Settings::default()
            }
        );
        let ranges = [
            ("long-press", KeySettings::HOLD_DELAY_MS),
            ("start repeating", KeySettings::REPEAT_DELAY_MS),
            ("Repeats per second", KeySettings::REPEAT_RATE),
            ("Pixels of drag", KeySettings::SWIPE_STEP_PX),
            ("Key height", KeySettings::HEIGHT_PX),
        ];
        for (comment, limits) in ranges {
            let line = TEMPLATE.lines().find(|l| l.contains(comment)).unwrap();
            assert!(
                line.contains(&format!("({}-{})", limits.min, limits.max)),
                "{line}"
            );
        }
    }

    #[test]
    fn a_new_file_starts_from_the_template() {
        let text = saved(None, |s| s.keys.hold_delay_ms = 450);
        assert!(text.contains("hold-delay = 450\n"), "{text}");
        assert!(
            text.contains("# Milliseconds before a long-press"),
            "{text}"
        );
        assert!(text.contains("[style.dark]"), "{text}");
    }

    #[test]
    fn edits_in_place_and_keeps_comments() {
        let source = r##"
# my settings
theme = "dark"   # easier on the eyes

[keys]
repeat-rate = 0x1e  # hex, because why not
height = 50 # tall keys

[style.dark]
accent = "#ff9e64"
"##;
        let text = saved(Some(source), |s| {
            s.theme = ThemeMode::Light;
            s.keys.height_px = None;
        });
        assert!(
            text.contains("theme = \"light\"   # easier on the eyes"),
            "{text}"
        );
        assert!(text.contains("height = \"auto\" # tall keys"), "{text}");
        assert!(
            text.contains("repeat-rate = 0x1e  # hex"),
            "unchanged values keep their form:\n{text}"
        );
        assert!(text.contains("# my settings"), "{text}");
        assert!(text.contains("accent = \"#ff9e64\""), "{text}");
        assert!(
            !text.contains("hold-delay"),
            "defaults are not added:\n{text}"
        );
    }

    #[test]
    fn adds_sections_and_inline_tables_as_needed() {
        let text = saved(Some("keys = { repeat-rate = 20 }\n"), |s| {
            s.keys.swipe_step_px = 20;
            s.preview.show_log = false;
            s.profile = Some("mine".into());
        });
        assert!(
            text.contains("keys = { repeat-rate = 20, swipe-step = 20 }"),
            "{text}"
        );
        assert!(text.contains("[preview]\nshow-log = false"), "{text}");
        assert!(text.contains("profile = \"mine\""), "{text}");

        let text = saved(Some("profile = \"mine\"\n"), |s| s.profile = None);
        assert!(!text.contains("profile"), "{text}");
    }

    #[test]
    fn refuses_to_edit_a_broken_file() {
        assert!(SettingsDocument::open(Some("theme = ")).is_err());
        assert!(SettingsDocument::open(Some("theme = \"blue\"")).is_err());
        assert!(SettingsDocument::open(Some("  \n")).is_ok());
    }

    #[test]
    fn writes_profiles_and_the_menu() {
        let mut spec = ProfileSpec::new("mine", "Mine", "dev");
        spec.sections.set_enabled("number-row", true, false);
        spec.sections.order = vec!["letters".into(), "brackets".into()];
        let text = saved(None, |s| {
            s.profile = Some("mine".into());
            s.profiles.push(spec.clone());
            s.menu.set_enabled("compact", false, true);
        });
        assert!(text.contains("profile = \"mine\""), "{text}");
        assert!(
            text.contains(
                "[[profiles]]\nname = \"mine\"\ntitle = \"Mine\"\nlayout = \"dev\"\n\
                 order = [\"letters\", \"brackets\"]\nsections = { number-row = true }\n"
            ),
            "{text}"
        );
        assert!(
            text.contains("[menu]\nshown = { compact = false }"),
            "{text}"
        );
    }

    #[test]
    fn edits_profiles_in_place_and_removes_deleted_ones() {
        let source = r#"
[[profiles]]
name = "a"   # first
layout = "dev"
sections = { brackets = false }  # no brackets

[[profiles]]
name = "b"
layout = "dev"

[menu]
order = ["b", "a"]
"#;
        let text = saved(Some(source), |s| {
            s.profiles[0].sections.set_enabled("brackets", true, true);
            s.profiles[0].title = "First".into();
            s.profiles.retain(|p| p.name != "b");
            s.menu.order.clear();
        });
        assert!(text.contains("name = \"a\"   # first"), "{text}");
        assert!(text.contains("title = \"First\""), "{text}");
        assert!(!text.contains("sections"), "back to defaults:\n{text}");
        assert!(!text.contains("name = \"b\""), "{text}");
        assert!(!text.contains("order"), "{text}");
    }

    #[test]
    fn inline_profile_arrays_become_tables() {
        let text = saved(
            Some("profiles = [{ name = \"a\", layout = \"dev\" }]\n"),
            |s| s.profiles[0].layout = "vim".into(),
        );
        assert!(text.contains("[[profiles]]"), "{text}");
        assert!(text.contains("layout = \"vim\""), "{text}");
    }
}
