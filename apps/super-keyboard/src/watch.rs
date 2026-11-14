//! Live reload: picks up edits to `settings.toml`, `layouts/*.toml` and
//! `themes/*.json`.

use std::time::Duration;

use dioxus::prelude::*;
use super_keyboard_config::Fingerprint;

use crate::{
    state::{AppEnv, AppState},
    time,
};

/// How often the config directory is checked for changes.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Reloads the configuration whenever its files change, if `env` asks for it.
pub fn use_config_watcher(mut state: AppState, env: &AppEnv) {
    let dir = env.config_dir.clone().filter(|_| env.watch);
    use_hook(move || {
        let Some(dir) = dir else {
            return;
        };
        spawn(async move {
            let mut last = Fingerprint::scan(&dir);
            loop {
                time::sleep(POLL_INTERVAL).await;
                let current = Fingerprint::scan(&dir);
                if current != last {
                    last = current;
                    state.reload();
                }
            }
        });
    });
}
