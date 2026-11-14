//! Loading, reloading and saving the whole configuration directory.

use std::{fmt, io};

use super_keyboard_core::{Keymap, Layout};

use crate::{
    ConfigDir, LayoutEntry, LayoutId, LayoutLibrary, LayoutOrigin, Settings, SettingsError,
    profile::{DEFAULT_PROFILE, Profile, ProfileCatalog, presets},
    settings::SettingsDocument,
};

const SETTINGS_FILE: &str = "settings.toml";

/// Something wrong with one config file. Problems are shown to the user; they
/// never stop the keyboard from working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The file, relative to the config directory.
    pub file: String,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

/// Why settings could not be saved. The new settings still apply until the app
/// closes.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("there is no config directory on this platform, so settings last until the app closes")]
    NoConfigDir,
    #[error("settings.toml has an error; fix it before saving from the app: {0}")]
    BrokenFile(#[from] SettingsError),
    #[error("could not write settings.toml: {0}")]
    Io(#[from] io::Error),
}

/// Settings, layouts and profiles, as loaded from a [`ConfigDir`].
#[derive(Debug, Clone, PartialEq)]
pub struct UserConfig {
    pub dir: Option<ConfigDir>,
    pub settings: Settings,
    pub library: LayoutLibrary,
    /// Presets, the user's profiles and one per otherwise unused layout. Rebuilt
    /// whenever settings or layouts change.
    pub profiles: ProfileCatalog,
    pub problems: Vec<Problem>,
}

impl Default for UserConfig {
    /// Built-in layouts, presets and default settings, without a config directory.
    fn default() -> Self {
        let library = LayoutLibrary::builtin();
        Self {
            dir: None,
            settings: Settings::default(),
            profiles: ProfileCatalog::new(presets(), &[], &library),
            library,
            problems: Vec::new(),
        }
    }
}

impl UserConfig {
    /// Loads `settings.toml` and `layouts/*.toml` from `dir`. Missing files are
    /// fine; broken ones are skipped and reported in [`UserConfig::problems`].
    pub fn load(dir: Option<ConfigDir>) -> Self {
        Self::load_over(dir, None)
    }

    /// Loads again from the same directory.
    ///
    /// A file that is now broken keeps its previous good version, so saving a
    /// half-finished edit does not reset the keyboard; the problem is still
    /// reported.
    #[must_use]
    pub fn reload(&self) -> Self {
        Self::load_over(self.dir.clone(), Some(self))
    }

    /// The profile the settings ask for, or the default profile if it is missing.
    pub fn active_profile(&self) -> &Profile {
        self.settings
            .profile
            .as_deref()
            .and_then(|name| self.profiles.get(name))
            .or_else(|| self.profiles.get(DEFAULT_PROFILE))
            .unwrap_or(&self.profiles.all()[0])
    }

    /// The active profile's layout, or the default layout if it is missing.
    pub fn active_layout(&self) -> &LayoutEntry {
        self.library
            .get(&self.active_profile().spec.layout)
            .unwrap_or_else(|| self.library.default_entry())
    }

    /// What the keyboard shows: the active layout, arranged by the active profile.
    pub fn keymap(&self) -> Keymap {
        self.active_layout()
            .layout
            .keymap(&self.active_profile().spec.sections)
    }

    /// Every profile in menu order, with whether the menu shows it.
    pub fn menu(&self) -> Vec<(&Profile, bool)> {
        self.profiles.menu(&self.settings.menu)
    }

    /// Applies `settings` and saves them to `settings.toml`, keeping the file's
    /// comments. The settings apply even if saving fails.
    pub fn update_settings(&mut self, settings: Settings) -> Result<(), SaveError> {
        self.settings = settings;
        self.problems.retain(|p| p.file != SETTINGS_FILE);
        self.refresh_profiles();

        let dir = self.dir.as_ref().ok_or(SaveError::NoConfigDir)?;
        let mut doc = SettingsDocument::open(dir.read_settings()?.as_deref())?;
        doc.apply(&self.settings);
        dir.write_settings(&doc.to_string())?;
        Ok(())
    }

    fn load_over(dir: Option<ConfigDir>, previous: Option<&Self>) -> Self {
        let mut config = Self {
            dir: dir.clone(),
            ..Self::default()
        };
        if let Some(dir) = dir {
            config.load_settings(&dir, previous);
            config.load_layouts(&dir, previous);
            config.refresh_profiles();
        }
        config
    }

    fn load_settings(&mut self, dir: &ConfigDir, previous: Option<&Self>) {
        let loaded = match dir.read_settings() {
            Ok(None) => return,
            Ok(Some(source)) => Settings::from_toml(&source).map_err(|err| err.to_string()),
            Err(err) => Err(err.to_string()),
        };
        match loaded {
            Ok(settings) => self.settings = settings,
            Err(message) => {
                if let Some(previous) = previous {
                    self.settings = previous.settings.clone();
                }
                self.problem(SETTINGS_FILE, message);
            }
        }
    }

    fn load_layouts(&mut self, dir: &ConfigDir, previous: Option<&Self>) {
        let files = match dir.layout_files() {
            Ok(files) => files,
            Err(err) => return self.problem("layouts", err.to_string()),
        };
        for path in files {
            let shown = dir.display_path(&path);
            let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
                self.problem(&shown, "the file name is not valid UTF-8".into());
                continue;
            };
            let id = LayoutId::new(id);
            let origin = LayoutOrigin::File(path.clone());
            let loaded = ConfigDir::read_file(&path)
                .map_err(|err| err.to_string())
                .and_then(|source| Layout::from_toml(&source).map_err(|err| err.to_string()));
            match loaded {
                Ok(layout) => self.library.insert(LayoutEntry { id, layout, origin }),
                Err(message) => {
                    let last_good = previous
                        .and_then(|p| p.library.get(id.as_str()))
                        .filter(|entry| entry.origin == origin);
                    if let Some(entry) = last_good {
                        self.library.insert(entry.clone());
                    }
                    self.problem(&shown, message);
                }
            }
        }
    }

    /// Rebuilds the profile catalog and reports profiles that cannot be shown as
    /// written. Presets are not checked: they may name sections that a user's
    /// replacement `dev.toml` does not have, and that is not a mistake.
    fn refresh_profiles(&mut self) {
        self.profiles = ProfileCatalog::new(presets(), &self.settings.profiles, &self.library);
        let mut problems = Vec::new();
        if let Some(name) = &self.settings.profile
            && self.profiles.get(name).is_none()
        {
            problems.push(format!(
                "profile `{name}` not found; using `{DEFAULT_PROFILE}`"
            ));
        }
        for profile in self.profiles.all().iter().filter(|p| p.is_saved()) {
            let (name, layout) = (profile.name(), &profile.spec.layout);
            match self.library.get(layout) {
                None => problems.push(format!(
                    "profile `{name}` uses unknown layout `{layout}`; showing `{}`",
                    LayoutLibrary::DEFAULT
                )),
                Some(entry) => {
                    let unknown = entry.layout.unknown_sections(&profile.spec.sections);
                    if !unknown.is_empty() {
                        problems.push(format!(
                            "profile `{name}`: layout `{layout}` has no section `{}`",
                            unknown.join("`, `")
                        ));
                    }
                }
            }
        }
        for message in problems {
            self.problem(SETTINGS_FILE, message);
        }
    }

    fn problem(&mut self, file: &str, message: String) {
        self.problems.push(Problem {
            file: file.to_owned(),
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::ThemeMode;

    const MINE: &str = "name = \"mine\"\n[[layers]]\nname = \"base\"\nrows = [[\"x\"]]\n";

    struct Fixture {
        _tmp: tempfile::TempDir,
        dir: ConfigDir,
    }

    impl Fixture {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let dir = ConfigDir::new(tmp.path().join("super-keyboard"));
            fs::create_dir_all(dir.layouts_dir()).unwrap();
            Self { _tmp: tmp, dir }
        }

        fn layout(&self, name: &str, source: &str) {
            fs::write(self.dir.layouts_dir().join(name), source).unwrap();
        }

        fn settings(&self, source: &str) {
            fs::write(self.dir.settings_file(), source).unwrap();
        }

        fn load(&self) -> UserConfig {
            UserConfig::load(Some(self.dir.clone()))
        }
    }

    fn ids(config: &UserConfig) -> Vec<&str> {
        config
            .library
            .entries()
            .iter()
            .map(|e| e.id.as_str())
            .collect()
    }

    #[test]
    fn without_a_directory_everything_is_built_in() {
        let config = UserConfig::load(None);
        assert_eq!(config, UserConfig::default());
        assert_eq!(config.active_layout().id.as_str(), "dev");
    }

    #[test]
    fn loads_settings_and_layouts() {
        let fx = Fixture::new();
        fx.layout("mine.toml", MINE);
        fx.settings("profile = \"mine\"\ntheme = \"dark\"\n");
        let config = fx.load();
        assert_eq!(config.problems, []);
        assert_eq!(ids(&config), ["dev", "mine"]);
        assert_eq!(config.settings.theme, ThemeMode::Dark);
        assert_eq!(config.active_layout().layout.name, "mine");
    }

    #[test]
    fn broken_files_become_problems() {
        let fx = Fixture::new();
        fx.layout(
            "bad.toml",
            "name = \"bad\"\n[[layers]]\nname = \"a\"\nrows = [[\"<nope>\"]]",
        );
        fx.settings("profile = \"missing\"\n");
        let config = fx.load();
        assert_eq!(ids(&config), ["dev"]);
        let problems: Vec<String> = config.problems.iter().map(ToString::to_string).collect();
        let bad = format!("layouts{}bad.toml", std::path::MAIN_SEPARATOR);
        assert_eq!(
            problems,
            [
                format!(
                    "{bad}: layer `a`, row 1, key 1: unknown key `<nope>` (write {{ text = \"<nope>\" }} to insert it as text)"
                ),
                "settings.toml: profile `missing` not found; using `coding`".to_owned(),
            ]
        );
        assert_eq!(config.active_layout().id.as_str(), "dev");

        fx.settings("theme = \"blue\"");
        let config = fx.load();
        assert_eq!(config.settings, Settings::default());
        assert!(config.problems.iter().any(|p| p.file == "settings.toml"));
    }

    #[test]
    fn reload_keeps_the_last_good_version_of_broken_files() {
        let fx = Fixture::new();
        fx.layout("mine.toml", MINE);
        fx.settings("theme = \"dark\"");
        let config = fx.load();

        fx.layout("mine.toml", "name = \"mine\"\n[[layers]]\nname = ");
        fx.settings("theme = ");
        let reloaded = config.reload();
        assert_eq!(reloaded.library, config.library);
        assert_eq!(reloaded.settings.theme, ThemeMode::Dark);
        assert_eq!(reloaded.problems.len(), 2);

        // Deleting a file is not breaking it: the layout goes away.
        fs::remove_file(fx.dir.layouts_dir().join("mine.toml")).unwrap();
        assert_eq!(ids(&config.reload()), ["dev"]);
    }

    #[test]
    fn a_user_dev_layout_replaces_the_builtin() {
        let fx = Fixture::new();
        fx.layout("dev.toml", MINE);
        let config = fx.load();
        assert_eq!(ids(&config), ["dev"]);
        assert_eq!(config.active_layout().layout.name, "mine");
    }

    #[test]
    fn update_settings_saves_and_clears_settings_problems() {
        let fx = Fixture::new();
        fx.settings("# keep me\nprofile = \"missing\"\n");
        let mut config = fx.load();
        assert_eq!(config.problems.len(), 1);

        let mut settings = config.settings.clone();
        settings.profile = Some("terminal".into());
        settings.keys.repeat_rate = 45;
        config.update_settings(settings.clone()).unwrap();
        assert_eq!(config.problems, []);

        let saved = fs::read_to_string(fx.dir.settings_file()).unwrap();
        assert!(
            saved.starts_with("# keep me\nprofile = \"terminal\"\n"),
            "{saved}"
        );
        assert_eq!(fx.load().settings, settings);
    }

    #[test]
    fn update_settings_creates_the_file_from_the_template() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path().join("new"));
        let mut config = UserConfig::load(Some(dir.clone()));
        let mut settings = config.settings.clone();
        settings.theme = ThemeMode::Light;
        config.update_settings(settings).unwrap();
        let saved = fs::read_to_string(dir.settings_file()).unwrap();
        assert!(saved.contains("theme = \"light\""), "{saved}");
        assert!(saved.contains("[style.dark]"), "{saved}");
    }

    #[test]
    fn update_settings_applies_even_when_it_cannot_save() {
        let mut config = UserConfig::default();
        let mut settings = Settings::default();
        settings.preview.show_log = false;
        let err = config.update_settings(settings.clone()).unwrap_err();
        assert!(matches!(err, SaveError::NoConfigDir));
        assert_eq!(config.settings, settings);

        let fx = Fixture::new();
        fx.settings("theme = ");
        let mut config = fx.load();
        let err = config.update_settings(settings).unwrap_err();
        assert!(matches!(err, SaveError::BrokenFile(_)), "{err}");
        assert_eq!(
            fs::read_to_string(fx.dir.settings_file()).unwrap(),
            "theme = "
        );
    }

    /// The labels of the first row of the keymap's base layer.
    fn first_row(config: &UserConfig) -> String {
        config.keymap().layers[0].rows[0]
            .iter()
            .map(|k| k.label.as_str())
            .collect()
    }

    #[test]
    fn profiles_arrange_the_keymap() {
        let fx = Fixture::new();
        assert_eq!(first_row(&fx.load()), "esctab(){}[];:");

        fx.settings(
            "profile = \"mine\"\n\
             [[profiles]]\nname = \"mine\"\nlayout = \"dev\"\n\
             order = [\"letters\", \"brackets\"]\nsections = { number-row = true }\n",
        );
        let config = fx.load();
        assert_eq!(config.problems, []);
        assert_eq!(config.active_profile().title(), "mine");
        // The number row, then letters before brackets.
        assert_eq!(first_row(&config), "1234567890");
        let base = &config.keymap().layers[0];
        assert_eq!(base.rows[1][0].label, "q");
        assert_eq!(base.rows[4][2].label, "(");

        fx.settings("profile = \"terminal\"\n");
        assert_eq!(first_row(&fx.load()), "esctab~|/-←↓↑→");
    }

    #[test]
    fn layout_files_get_a_profile_and_the_menu_lists_everything() {
        let fx = Fixture::new();
        fx.layout("mine.toml", MINE);
        // `mine` and `coding` trade places; the others stay put.
        fx.settings("[menu]\norder = [\"mine\", \"coding\"]\nshown = { compact = false }\n");
        let config = fx.load();
        let menu: Vec<_> = config
            .menu()
            .into_iter()
            .map(|(p, shown)| format!("{}{}", p.name(), if shown { "" } else { "-" }))
            .collect();
        assert_eq!(
            menu,
            ["mine", "coding-numbers", "terminal", "compact-", "coding"]
        );
    }

    #[test]
    fn reports_profiles_that_cannot_be_shown_as_written() {
        let fx = Fixture::new();
        fx.settings(
            "[[profiles]]\nname = \"a\"\nlayout = \"vim\"\n\
             [[profiles]]\nname = \"b\"\nlayout = \"dev\"\nsections = { nope = true, extra = false }\n",
        );
        let problems: Vec<String> = fx.load().problems.iter().map(ToString::to_string).collect();
        assert_eq!(
            problems,
            [
                "settings.toml: profile `a` uses unknown layout `vim`; showing `dev`",
                "settings.toml: profile `b`: layout `dev` has no section `extra`, `nope`",
            ]
        );
    }

    #[test]
    fn changing_a_preset_saves_a_copy_and_removing_it_restores_the_preset() {
        let fx = Fixture::new();
        let mut config = fx.load();
        let mut settings = config.settings.clone();
        let mut terminal = config.profiles.get("terminal").unwrap().spec.clone();
        terminal.sections.set_enabled("number-row", true, false);
        settings.put_profile(terminal);
        settings.profile = Some("terminal".into());
        config.update_settings(settings).unwrap();
        assert_eq!(
            config.active_profile().origin,
            crate::ProfileOrigin::CustomizedPreset
        );
        assert_eq!(first_row(&config), "1234567890");

        let mut settings = config.settings.clone();
        assert!(settings.remove_profile("terminal"));
        config.update_settings(settings).unwrap();
        assert_eq!(config.active_profile().origin, crate::ProfileOrigin::Preset);
        assert_eq!(first_row(&fx.load()), "esctab~|/-←↓↑→");
    }
}
