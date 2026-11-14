//! Profiles: a layout plus an arrangement of its sections, and the presets that
//! ship with the app.
//!
//! ```toml
//! # settings.toml
//! profile = "mine"
//!
//! [[profiles]]
//! name = "mine"
//! title = "Mine"
//! layout = "dev"
//! order = ["letters", "brackets"]            # these sections in this order
//! sections = { number-row = true, brackets = false }
//! ```

use std::{collections::BTreeMap, sync::OnceLock};

use serde::Deserialize;
use super_keyboard_core::Arrangement;

use crate::{LayoutLibrary, SettingsError};

/// The built-in profiles.
pub const PRESETS_TOML: &str = include_str!("../presets.toml");

/// The profile used when settings do not name one, or name a missing one.
pub const DEFAULT_PROFILE: &str = "coding";

/// A profile as written in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSpec {
    /// How settings refer to it: letters, digits, `-` and `_`.
    pub name: String,
    /// What menus call it.
    pub title: String,
    /// The id of the layout it uses.
    pub layout: String,
    /// Which of the layout's sections are on, and their order.
    pub sections: Arrangement,
}

impl ProfileSpec {
    /// A profile showing `layout` with its default sections.
    pub fn new(name: &str, title: &str, layout: &str) -> Self {
        Self {
            name: name.to_owned(),
            title: title.to_owned(),
            layout: layout.to_owned(),
            sections: Arrangement::default(),
        }
    }
}

/// Where a profile comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileOrigin {
    /// Ships with the app.
    Preset,
    /// A preset the user changed. Removing the change brings the preset back.
    CustomizedPreset,
    /// Made by the user.
    User,
    /// Made automatically for a layout that no other profile uses.
    Layout,
}

impl ProfileOrigin {
    /// A short tag for menus and settings screens.
    pub fn label(self) -> &'static str {
        match self {
            Self::Preset => "preset",
            Self::CustomizedPreset => "changed preset",
            Self::User => "yours",
            Self::Layout => "layout",
        }
    }
}

/// A profile that can be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub spec: ProfileSpec,
    pub origin: ProfileOrigin,
}

impl Profile {
    pub fn name(&self) -> &str {
        &self.spec.name
    }

    pub fn title(&self) -> &str {
        &self.spec.title
    }

    /// Whether settings hold this profile, so editing it changes the user's copy.
    pub fn is_saved(&self) -> bool {
        matches!(
            self.origin,
            ProfileOrigin::User | ProfileOrigin::CustomizedPreset
        )
    }
}

/// Every profile that can be chosen, in their default menu order: presets (or
/// the user's changes to them), the user's own profiles, then one profile for
/// each layout that no other profile uses, so a new layout file is always
/// reachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileCatalog {
    profiles: Vec<Profile>,
    presets: Vec<ProfileSpec>,
}

impl ProfileCatalog {
    pub fn new(presets: &[ProfileSpec], user: &[ProfileSpec], library: &LayoutLibrary) -> Self {
        let mut profiles: Vec<Profile> = presets
            .iter()
            .map(|preset| match user.iter().find(|u| u.name == preset.name) {
                Some(changed) => Profile {
                    spec: changed.clone(),
                    origin: ProfileOrigin::CustomizedPreset,
                },
                None => Profile {
                    spec: preset.clone(),
                    origin: ProfileOrigin::Preset,
                },
            })
            .collect();
        for spec in user {
            if !presets.iter().any(|preset| preset.name == spec.name) {
                profiles.push(Profile {
                    spec: spec.clone(),
                    origin: ProfileOrigin::User,
                });
            }
        }
        for entry in library.entries() {
            let id = entry.id.as_str();
            let covered = profiles
                .iter()
                .any(|p| p.spec.layout == id || p.spec.name == id);
            if !covered {
                profiles.push(Profile {
                    spec: ProfileSpec::new(id, id, id),
                    origin: ProfileOrigin::Layout,
                });
            }
        }
        Self {
            profiles,
            presets: presets.to_vec(),
        }
    }

    pub fn all(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn get(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name() == name)
    }

    /// Profile names in their default menu order.
    pub fn names(&self) -> Vec<&str> {
        self.profiles.iter().map(Profile::name).collect()
    }

    /// The preset called `name`, as shipped.
    pub fn preset(&self, name: &str) -> Option<&ProfileSpec> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// Every profile in menu order, with whether the menu shows it.
    pub fn menu(&self, menu: &Arrangement) -> Vec<(&Profile, bool)> {
        menu.arrange(&self.names())
            .into_iter()
            .map(|i| {
                let profile = &self.profiles[i];
                (profile, menu.is_enabled(profile.name(), true))
            })
            .collect()
    }

    /// A name no profile has yet, such as `profile-2`.
    pub fn unused_name(&self) -> String {
        (1..)
            .map(|n| format!("profile-{n}"))
            .find(|name| self.get(name).is_none())
            .expect("there is always an unused name")
    }
}

/// The presets that ship with the app, parsed once.
pub fn presets() -> &'static [ProfileSpec] {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PresetsFile {
        profiles: Vec<ProfileFile>,
    }
    static PRESETS: OnceLock<Vec<ProfileSpec>> = OnceLock::new();
    PRESETS.get_or_init(|| {
        let file: PresetsFile = toml::from_str(PRESETS_TOML).expect("presets.toml is valid TOML");
        validate_profiles(file.profiles).expect("presets.toml is valid")
    })
}

/// A profile as written in TOML.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileFile {
    name: String,
    title: Option<String>,
    layout: String,
    #[serde(default)]
    order: Vec<String>,
    #[serde(default)]
    sections: BTreeMap<String, bool>,
}

pub(crate) fn validate_profiles(
    files: Vec<ProfileFile>,
) -> Result<Vec<ProfileSpec>, SettingsError> {
    let mut specs: Vec<ProfileSpec> = Vec::with_capacity(files.len());
    for file in files {
        if !is_valid_name(&file.name) {
            return Err(SettingsError::InvalidProfileName(file.name));
        }
        if specs.iter().any(|s| s.name == file.name) {
            return Err(SettingsError::DuplicateProfile(file.name));
        }
        if file.layout.trim().is_empty() {
            return Err(SettingsError::EmptyProfileLayout(file.name));
        }
        specs.push(ProfileSpec {
            title: file.title.unwrap_or_else(|| file.name.clone()),
            name: file.name,
            layout: file.layout,
            sections: Arrangement {
                order: file.order,
                enabled: file.sections,
            },
        });
    }
    Ok(specs)
}

/// Profile names, like section names, are simple identifiers.
pub(crate) fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutEntry, LayoutId, LayoutOrigin};
    use super_keyboard_core::Layout;

    fn names(profiles: &[(&Profile, bool)]) -> Vec<String> {
        profiles
            .iter()
            .map(|(p, shown)| format!("{}{}", p.name(), if *shown { "" } else { " (hidden)" }))
            .collect()
    }

    fn library_with(ids: &[&str]) -> LayoutLibrary {
        let mut library = LayoutLibrary::builtin();
        for id in ids {
            library.insert(LayoutEntry {
                id: LayoutId::new(*id),
                layout: Layout::builtin(),
                origin: LayoutOrigin::File(format!("{id}.toml").into()),
            });
        }
        library
    }

    #[test]
    fn presets_only_name_sections_of_the_builtin_layout() {
        let layout = Layout::builtin();
        let presets = presets();
        assert_eq!(presets[0].name, DEFAULT_PROFILE);
        for preset in presets {
            assert_eq!(preset.layout, "dev");
            assert!(
                layout.unknown_sections(&preset.sections).is_empty(),
                "{}: {:?}",
                preset.name,
                layout.unknown_sections(&preset.sections)
            );
        }
    }

    #[test]
    fn catalog_merges_presets_user_profiles_and_layouts() {
        let mut changed = presets()[2].clone();
        changed.title = "My terminal".into();
        let user = [changed, ProfileSpec::new("vim", "Vim", "dev")];
        let catalog = ProfileCatalog::new(presets(), &user, &library_with(&["vim", "numpad"]));
        let all: Vec<_> = catalog
            .all()
            .iter()
            .map(|p| format!("{}:{}", p.name(), p.origin.label()))
            .collect();
        assert_eq!(
            all,
            [
                "coding:preset",
                "coding-numbers:preset",
                "terminal:changed preset",
                "compact:preset",
                "vim:yours",
                // `vim.toml` has a profile of that name already; `numpad.toml` has none.
                "numpad:layout",
            ]
        );
        assert_eq!(catalog.get("terminal").unwrap().title(), "My terminal");
        assert_eq!(catalog.preset("terminal").unwrap().title, "Terminal");
        assert!(catalog.get("terminal").unwrap().is_saved());
        assert!(!catalog.get("coding").unwrap().is_saved());
    }

    #[test]
    fn the_menu_can_be_reordered_and_entries_hidden() {
        let catalog = ProfileCatalog::new(presets(), &[], &LayoutLibrary::builtin());
        let mut menu = Arrangement::default();
        menu.move_item(&catalog.names(), "compact", -3);
        menu.set_enabled("coding-numbers", false, true);
        assert_eq!(
            names(&catalog.menu(&menu)),
            ["compact", "coding", "coding-numbers (hidden)", "terminal"]
        );
    }

    #[test]
    fn unused_names_skip_existing_profiles() {
        let user = [ProfileSpec::new("profile-1", "1", "dev")];
        let catalog = ProfileCatalog::new(presets(), &user, &LayoutLibrary::builtin());
        assert_eq!(catalog.unused_name(), "profile-2");
    }

    #[test]
    fn profile_names_are_identifiers() {
        assert!(is_valid_name("my-profile_2"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("my profile"));
        assert!(!is_valid_name("é"));
    }
}
