//! Settings shared with the Super Keyboard app, through `settings.toml`.
//!
//! Both sides read and write the same `[vietnamese]` section and both watch
//! the file, so switching VI / EN or Telex / VNI on either side shows on the
//! other within a second. Writing goes through `super-keyboard-config`, which
//! keeps the file's comments.

use std::sync::{Arc, Mutex, MutexGuard};

use super_keyboard_config::{ConfigDir, Fingerprint, UserConfig};
use super_keyboard_core::VietnameseOptions;

/// The settings, shared by every engine instance and the watcher.
#[derive(Debug, Clone)]
pub struct Shared(Arc<Mutex<State>>);

#[derive(Debug)]
struct State {
    config: UserConfig,
    fingerprint: Option<Fingerprint>,
}

impl Shared {
    /// Loads `~/.config/super-keyboard` (or `$XDG_CONFIG_HOME`).
    pub fn load() -> Self {
        Self::load_from(ConfigDir::detect())
    }

    pub fn load_from(dir: Option<ConfigDir>) -> Self {
        let fingerprint = dir.as_ref().map(Fingerprint::scan);
        Self(Arc::new(Mutex::new(State {
            config: UserConfig::load(dir),
            fingerprint,
        })))
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // A panic while holding the lock leaves plain data behind; keep going.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn options(&self) -> VietnameseOptions {
        self.state().config.settings.vietnamese
    }

    /// Reads the files again if they changed. Returns whether the Vietnamese
    /// options changed.
    pub fn poll(&self) -> bool {
        let mut state = self.state();
        let Some(dir) = state.config.dir.clone() else {
            return false;
        };
        let current = Fingerprint::scan(&dir);
        if state.fingerprint.as_ref() == Some(&current) {
            return false;
        }
        state.fingerprint = Some(current);
        let before = state.config.settings.vietnamese;
        state.config = state.config.reload();
        state.config.settings.vietnamese != before
    }

    /// Changes the options and saves them. They apply even when they cannot be
    /// saved (no config directory, or a `settings.toml` with mistakes in it);
    /// the error says why.
    pub fn update(&self, change: impl FnOnce(&mut VietnameseOptions)) -> Result<(), String> {
        let mut state = self.state();
        let mut settings = state.config.settings.clone();
        change(&mut settings.vietnamese);
        if settings == state.config.settings {
            return Ok(());
        }
        let saved = state.config.update_settings(settings);
        // Our own write is not news to the watcher.
        state.fingerprint = state.config.dir.as_ref().map(Fingerprint::scan);
        saved.map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use super_keyboard_core::InputMethod;

    #[test]
    fn edits_on_either_side_are_seen_by_the_other() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path());
        fs::write(
            dir.settings_file(),
            "# mine\n[vietnamese]\nenabled = true\n",
        )
        .unwrap();

        let engine = Shared::load_from(Some(dir.clone()));
        assert!(engine.options().enabled);
        assert!(!engine.poll(), "nothing changed yet");

        // The app (or a text editor) switches to VNI.
        fs::write(
            dir.settings_file(),
            "# mine\n[vietnamese]\nenabled = true\nmethod = \"vni\"\n",
        )
        .unwrap();
        assert!(engine.poll());
        assert_eq!(engine.options().method, InputMethod::Vni);

        // The engine's menu switches Vietnamese off; the file keeps comments.
        engine.update(|o| o.enabled = false).unwrap();
        assert!(!engine.poll(), "its own write is not a change");
        let saved = fs::read_to_string(dir.settings_file()).unwrap();
        assert!(saved.starts_with("# mine\n"), "{saved}");
        assert!(saved.contains("enabled = false"), "{saved}");
        let app = UserConfig::load(Some(dir));
        assert!(!app.settings.vietnamese.enabled);
        assert_eq!(app.settings.vietnamese.method, InputMethod::Vni);
    }

    #[test]
    fn changes_apply_without_a_config_directory() {
        let shared = Shared::load_from(None);
        assert!(shared.update(|o| o.enabled = true).is_err());
        assert!(shared.options().enabled);
        assert!(!shared.poll());
    }
}
