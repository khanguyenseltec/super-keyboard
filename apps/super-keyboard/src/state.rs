//! App-wide state, shared with components through context.

use dioxus::prelude::*;
use super_keyboard_config::{
    Appearance, ConfigDir, ProfileOrigin, ProfileSpec, Settings, ThemeChoice, UserConfig,
};
use super_keyboard_core::{Arrangement, Layout};

use crate::controller::KeyboardController;

/// Where the app finds its configuration. Provided as context at launch so tests
/// can run against a temporary directory instead of the user's real one.
#[derive(Debug, Clone, PartialEq)]
pub struct AppEnv {
    pub config_dir: Option<ConfigDir>,
    /// Reload the config directory when its files change.
    pub watch: bool,
}

impl AppEnv {
    /// The platform's config directory, watched on desktop. Android and the web
    /// preview load it once: polling would cost battery for files that only the
    /// app itself changes there.
    pub fn detect() -> Self {
        Self {
            config_dir: ConfigDir::detect(),
            watch: cfg!(not(any(target_os = "android", target_arch = "wasm32"))),
        }
    }

    /// No config directory: built-in layouts and default settings.
    #[cfg(test)]
    pub fn isolated() -> Self {
        Self {
            config_dir: None,
            watch: false,
        }
    }
}

/// What the area above the keyboard shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Panel {
    #[default]
    Preview,
    Settings,
}

/// Everything components share. Cheap to copy: every field is a handle.
#[derive(Clone, Copy, PartialEq)]
pub struct AppState {
    pub config: Signal<UserConfig>,
    pub keyboard: KeyboardController,
    pub panel: Signal<Panel>,
    /// Why the last settings change could not be saved, if it could not.
    pub save_error: Signal<Option<String>>,
}

impl AppState {
    /// Loads the configuration and creates the state in the current scope.
    pub fn load(env: &AppEnv) -> Self {
        let config = UserConfig::load(env.config_dir.clone());
        let mut keyboard = KeyboardController::new(
            config.keymap(),
            config.settings.keys.gesture_config(),
            config.settings.vietnamese,
        );
        keyboard.set_auto_pair(config.settings.keys.auto_pair);
        Self {
            config: Signal::new(config),
            keyboard,
            panel: Signal::new(Panel::default()),
            save_error: Signal::new(None),
        }
    }

    /// Reads the config directory again, after its files changed.
    pub fn reload(&mut self) {
        let config = self.config.peek().reload();
        if *self.config.peek() != config {
            self.config.set(config);
            self.sync_keyboard();
        }
    }

    /// Changes settings, applies them at once and saves them.
    pub fn update_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        let mut settings = self.config.peek().settings.clone();
        change(&mut settings);
        if settings == self.config.peek().settings {
            return;
        }
        let saved = self.config.write().update_settings(settings);
        self.save_error.set(saved.err().map(|err| err.to_string()));
        self.sync_keyboard();
    }

    pub fn select_profile(&mut self, name: String) {
        self.update_settings(|settings| settings.profile = Some(name));
    }

    /// Changes the active profile. A preset or a layout's profile becomes the
    /// user's own copy under the same name; "reset" removes the copy again.
    pub fn edit_profile(&mut self, change: impl FnOnce(&mut ProfileSpec, &Layout)) {
        let (mut spec, layout) = {
            let config = self.config.peek();
            (
                config.active_profile().spec.clone(),
                config.active_layout().layout.clone(),
            )
        };
        change(&mut spec, &layout);
        let name = spec.name.clone();
        self.update_settings(|settings| {
            settings.put_profile(spec);
            settings.profile = Some(name);
        });
    }

    /// Switches a section of the active profile on or off.
    pub fn set_section(&mut self, section: String, on: bool) {
        self.edit_profile(|spec, layout| {
            if let Some(default) = layout.section(&section).map(|s| s.enabled) {
                spec.sections.set_enabled(&section, on, default);
            }
        });
    }

    /// Moves a section of the active profile `by` places (negative is up).
    pub fn move_section(&mut self, section: String, by: isize) {
        self.edit_profile(|spec, layout| {
            spec.sections
                .move_item(&layout.section_names(), &section, by);
        });
    }

    /// Puts the active profile's sections back in their default state and order.
    pub fn reset_sections(&mut self) {
        self.edit_profile(|spec, _| spec.sections = Arrangement::default());
    }

    pub fn rename_profile(&mut self, title: String) {
        let title = title.trim().to_owned();
        if !title.is_empty() {
            self.edit_profile(|spec, _| spec.title = title);
        }
    }

    /// Points the active profile at another layout, with that layout's default
    /// sections.
    pub fn set_profile_layout(&mut self, layout: String) {
        self.edit_profile(|spec, _| {
            spec.layout = layout;
            spec.sections = Arrangement::default();
        });
    }

    /// Copies the active profile into a new one of the user's, and uses it.
    pub fn new_profile(&mut self) {
        let (mut spec, name) = {
            let config = self.config.peek();
            (
                config.active_profile().spec.clone(),
                config.profiles.unused_name(),
            )
        };
        spec.title = format!("{} copy", spec.title);
        spec.name = name.clone();
        self.update_settings(|settings| {
            settings.put_profile(spec);
            settings.profile = Some(name);
        });
    }

    /// Deletes one of the user's profiles, or undoes the user's changes to a
    /// preset or a layout's profile.
    pub fn remove_profile(&mut self, name: String) {
        let origin = self.config.peek().profiles.get(&name).map(|p| p.origin);
        self.update_settings(|settings| {
            settings.remove_profile(&name);
            if origin == Some(ProfileOrigin::User) && settings.profile.as_deref() == Some(&name) {
                settings.profile = None;
            }
        });
    }

    /// Shows or hides a profile in the menu.
    pub fn set_in_menu(&mut self, name: String, shown: bool) {
        self.update_settings(|settings| settings.menu.set_enabled(&name, shown, true));
    }

    /// Moves a profile `by` places in the menu (negative is up).
    pub fn move_in_menu(&mut self, name: String, by: isize) {
        let names: Vec<String> = {
            let config = self.config.peek();
            config
                .profiles
                .names()
                .into_iter()
                .map(str::to_owned)
                .collect()
        };
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        self.update_settings(|settings| settings.menu.move_item(&names, &name, by));
    }

    /// Uses theme `id`. While following the system, it becomes the theme for
    /// the system's light or dark mode, whichever `id` is made for.
    pub fn choose_theme(&mut self, id: String) {
        let Some(appearance) = self
            .config
            .peek()
            .themes
            .get(&id)
            .map(|entry| entry.theme.appearance)
        else {
            return;
        };
        self.update_settings(|settings| match settings.theme.choice {
            ThemeChoice::System => settings.theme.set_for_appearance(appearance, id),
            ThemeChoice::Named(_) => settings.theme.choice = ThemeChoice::Named(id),
        });
    }

    /// Follows the system's light and dark mode, or stops. Following starts
    /// with the theme in use in its slot; stopping keeps the dark theme.
    pub fn follow_system(&mut self, follow: bool) {
        let fixed = match &self.config.peek().settings.theme.choice {
            ThemeChoice::Named(name) => self
                .config
                .peek()
                .themes
                .get(name)
                .map(|entry| (entry.id.clone(), entry.theme.appearance)),
            ThemeChoice::System => None,
        };
        self.update_settings(|settings| {
            let theme = &mut settings.theme;
            if follow {
                if let Some((id, appearance)) = fixed {
                    theme.set_for_appearance(appearance, id);
                }
                theme.choice = ThemeChoice::System;
            } else if theme.choice == ThemeChoice::System {
                theme.choice =
                    ThemeChoice::Named(theme.for_appearance(Appearance::Dark).to_owned());
            }
        });
    }

    /// Switches Vietnamese typing on or off.
    pub fn toggle_vietnamese(&mut self) {
        self.update_settings(|settings| {
            settings.vietnamese.enabled = !settings.vietnamese.enabled;
        });
    }

    pub fn toggle_panel(&mut self, panel: Panel) {
        let next = if *self.panel.peek() == panel {
            Panel::Preview
        } else {
            panel
        };
        self.panel.set(next);
    }

    /// Brings the keyboard in line with the configuration.
    fn sync_keyboard(&mut self) {
        let (keymap, gestures, auto_pair, vietnamese) = {
            let config = self.config.peek();
            (
                config.keymap(),
                config.settings.keys.gesture_config(),
                config.settings.keys.auto_pair,
                config.settings.vietnamese,
            )
        };
        self.keyboard.set_keymap(keymap);
        self.keyboard.set_gesture_config(gestures);
        self.keyboard.set_auto_pair(auto_pair);
        self.keyboard.set_vietnamese(vietnamese);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Runs `test` with app state loaded from `env`, inside a Dioxus runtime.
    fn with_state(env: AppEnv, test: impl FnOnce(AppState)) {
        #[component]
        fn Root(env: AppEnv) -> Element {
            use_context_provider(|| AppState::load(&env));
            rsx! {}
        }
        let mut dom = VirtualDom::new_with_props(Root, RootProps { env });
        dom.rebuild_in_place();
        dom.in_scope(ScopeId::APP, || test(consume_context::<AppState>()));
    }

    fn first_row(state: &AppState) -> String {
        state.keyboard.keyboard.peek().keymap().layers[0].rows[0]
            .iter()
            .map(|k| k.label.as_str())
            .collect()
    }

    fn active(state: &AppState) -> (String, ProfileOrigin) {
        let config = state.config.peek();
        let profile = config.active_profile();
        (profile.name().to_owned(), profile.origin)
    }

    #[test]
    fn sections_switch_and_move_live_and_change_a_preset_into_a_copy() {
        with_state(AppEnv::isolated(), |mut state| {
            assert_eq!(first_row(&state), "esctab(){}[];:");
            state.set_section("number-row".into(), true);
            assert_eq!(first_row(&state), "1234567890");
            assert_eq!(
                active(&state),
                ("coding".into(), ProfileOrigin::CustomizedPreset)
            );
            // Without a config directory the change still applies; it says why it
            // was not saved.
            assert!(state.save_error.peek().is_some());

            state.move_section("bottom".into(), -100);
            assert_eq!(first_row(&state), "ctrlsymaltspace,.nav⏎");
            state.set_section("letters".into(), false);
            assert!(
                state.keyboard.keyboard.peek().keymap().layers[0]
                    .rows
                    .iter()
                    .any(|row| row[0].label == "q"),
                "required sections stay on"
            );

            state.reset_sections();
            state.remove_profile("coding".into());
            assert_eq!(active(&state), ("coding".into(), ProfileOrigin::Preset));
            assert_eq!(first_row(&state), "esctab(){}[];:");
        });
    }

    #[test]
    fn profiles_are_copied_renamed_chosen_and_deleted() {
        with_state(AppEnv::isolated(), |mut state| {
            state.select_profile("terminal".into());
            state.new_profile();
            assert_eq!(active(&state), ("profile-1".into(), ProfileOrigin::User));
            assert_eq!(
                state.config.peek().active_profile().title(),
                "Terminal copy"
            );
            assert_eq!(first_row(&state), "esctab~|/-←↓↑→");
            state.rename_profile("  Shell  ".into());
            assert_eq!(state.config.peek().active_profile().title(), "Shell");
            state.rename_profile("   ".into());
            assert_eq!(state.config.peek().active_profile().title(), "Shell");

            state.remove_profile("profile-1".into());
            assert_eq!(active(&state), ("coding".into(), ProfileOrigin::Preset));
        });
    }

    #[test]
    fn the_menu_reorders_and_hides_profiles() {
        with_state(AppEnv::isolated(), |mut state| {
            state.move_in_menu("compact".into(), -1);
            state.set_in_menu("coding-numbers".into(), false);
            let menu: Vec<String> = state
                .config
                .peek()
                .menu()
                .into_iter()
                .map(|(p, shown)| format!("{}{}", p.name(), if shown { "" } else { "-" }))
                .collect();
            assert_eq!(menu, ["coding", "coding-numbers-", "compact", "terminal"]);
        });
    }

    #[test]
    fn themes_are_chosen_per_appearance_while_following_the_system() {
        with_state(AppEnv::isolated(), |mut state| {
            state.choose_theme("nord".into());
            state.choose_theme("paper".into());
            state.choose_theme("no-such-theme".into());
            let theme = state.config.peek().settings.theme.clone();
            assert_eq!(theme.choice, ThemeChoice::System);
            assert_eq!(
                (theme.light.as_str(), theme.dark.as_str()),
                ("paper", "nord")
            );

            state.follow_system(false);
            assert_eq!(
                state.config.peek().settings.theme.choice,
                ThemeChoice::Named("nord".into())
            );
            state.choose_theme("solarized-light".into());
            state.follow_system(true);
            let theme = state.config.peek().settings.theme.clone();
            assert_eq!(theme.choice, ThemeChoice::System);
            assert_eq!(
                theme.light, "solarized-light",
                "the fixed theme keeps its slot"
            );
            assert_eq!(theme.dark, "nord");
        });
    }

    #[test]
    fn the_auto_pair_setting_reaches_the_keyboard() {
        with_state(AppEnv::isolated(), |mut state| {
            assert!(!state.keyboard.auto_pair());
            state.update_settings(|s| s.keys.auto_pair = true);
            assert!(state.keyboard.auto_pair());
        });
    }

    #[test]
    fn the_vietnamese_switch_reaches_the_keyboard() {
        with_state(AppEnv::isolated(), |mut state| {
            assert!(!state.keyboard.vietnamese.peek().options().enabled);
            state.toggle_vietnamese();
            assert!(state.config.peek().settings.vietnamese.enabled);
            assert!(state.keyboard.vietnamese.peek().options().enabled);
            state.update_settings(|s| s.vietnamese.method = super_keyboard_core::InputMethod::Vni);
            assert_eq!(
                state.keyboard.vietnamese.peek().options().method,
                super_keyboard_core::InputMethod::Vni
            );
        });
    }

    #[test]
    fn changes_are_saved_to_settings_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path());
        let env = AppEnv {
            config_dir: Some(dir.clone()),
            watch: false,
        };
        with_state(env.clone(), |mut state| {
            state.set_section("number-row".into(), true);
            state.move_in_menu("compact".into(), -3);
            assert_eq!(*state.save_error.peek(), None);
        });
        let saved = fs::read_to_string(dir.settings_file()).unwrap();
        assert!(
            saved.contains("sections = { number-row = true }"),
            "{saved}"
        );
        assert!(saved.contains("[menu]"), "{saved}");
        with_state(env, |state| assert_eq!(first_row(&state), "1234567890"));
    }
}
