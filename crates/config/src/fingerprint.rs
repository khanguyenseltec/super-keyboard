//! Cheap change detection for live reload.
//!
//! Polling a fingerprint once a second is simpler and more robust than file
//! system notifications for a handful of small files: it sees editors that save
//! by renaming, directories created after start-up, and network mounts alike.

use std::{fs, path::PathBuf, time::SystemTime};

use crate::ConfigDir;

/// The names, sizes and modification times of every config file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fingerprint(Vec<(PathBuf, u64, Option<SystemTime>)>);

impl Fingerprint {
    pub fn scan(dir: &ConfigDir) -> Self {
        let mut files = dir.layout_files().unwrap_or_default();
        files.extend(dir.theme_files().unwrap_or_default());
        files.push(dir.settings_file());
        Self(
            files
                .into_iter()
                .filter_map(|path| {
                    let meta = fs::metadata(&path).ok()?;
                    Some((path, meta.len(), meta.modified().ok()))
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_when_files_are_added_edited_or_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path());
        let empty = Fingerprint::scan(&dir);
        assert_eq!(empty, Fingerprint::default());

        dir.write_settings("theme = \"dark\"").unwrap();
        let with_settings = Fingerprint::scan(&dir);
        assert_ne!(with_settings, empty);
        assert_eq!(
            Fingerprint::scan(&dir),
            with_settings,
            "stable without changes"
        );

        fs::create_dir_all(dir.layouts_dir()).unwrap();
        fs::write(dir.layouts_dir().join("a.toml"), "x").unwrap();
        let with_layout = Fingerprint::scan(&dir);
        assert_ne!(with_layout, with_settings);

        // Same modification time is possible on coarse clocks; size still differs.
        fs::write(dir.layouts_dir().join("a.toml"), "xy").unwrap();
        assert_ne!(Fingerprint::scan(&dir), with_layout);

        fs::remove_file(dir.layouts_dir().join("a.toml")).unwrap();
        assert_eq!(Fingerprint::scan(&dir), with_settings);

        dir.create_theme_file("mine", "{}").unwrap();
        assert_ne!(Fingerprint::scan(&dir), with_settings);
    }
}
