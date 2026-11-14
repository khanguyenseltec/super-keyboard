//! The configuration directory and the files in it.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// The directory that holds `settings.toml`, `layouts/` and `themes/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigDir {
    root: PathBuf,
}

/// Files larger than this are not loaded. Layouts and settings are a few KiB.
const MAX_FILE_BYTES: u64 = 512 * 1024;

impl ConfigDir {
    /// Overrides the configuration directory, for testing or portable setups.
    pub const ENV_VAR: &str = "SUPER_KEYBOARD_CONFIG_DIR";

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The configuration directory for this platform:
    ///
    /// 1. `$SUPER_KEYBOARD_CONFIG_DIR`, if set and not empty;
    /// 2. on Linux, `$XDG_CONFIG_HOME/super-keyboard` (usually
    ///    `~/.config/super-keyboard`), and the usual places on other desktops;
    /// 3. on Android, the app's private files directory.
    ///
    /// Returns `None` where there is no file system to use (the web preview).
    /// The directory does not have to exist yet.
    pub fn detect() -> Option<Self> {
        if let Some(root) = std::env::var_os(Self::ENV_VAR).filter(|v| !v.is_empty()) {
            return Some(Self::new(root));
        }
        platform_dir().map(Self::new)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.toml")
    }

    pub fn layouts_dir(&self) -> PathBuf {
        self.root.join("layouts")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.root.join("themes")
    }

    /// A path for messages: relative to the config directory when inside it.
    pub fn display_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string()
    }

    /// The contents of `settings.toml`, or `None` if there is none.
    pub fn read_settings(&self) -> io::Result<Option<String>> {
        read_optional(&self.settings_file())
    }

    /// Replaces `settings.toml` atomically, creating the directory if needed, so
    /// a crash or a watcher never sees a half-written file.
    pub fn write_settings(&self, contents: &str) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        let target = self.settings_file();
        let temporary = self.root.join(".settings.toml.tmp");
        fs::write(&temporary, contents)?;
        fs::rename(&temporary, &target)
    }

    /// The `*.toml` files in `layouts/`, sorted by path. Missing directory: none.
    pub fn layout_files(&self) -> io::Result<Vec<PathBuf>> {
        files_with_extension(&self.layouts_dir(), "toml")
    }

    /// The `*.json` files in `themes/`, sorted by path. Missing directory: none.
    pub fn theme_files(&self) -> io::Result<Vec<PathBuf>> {
        files_with_extension(&self.themes_dir(), "json")
    }

    /// Writes `themes/<id>.json`, creating the directory if needed. An existing
    /// file is never overwritten: it may hold the user's work.
    pub fn create_theme_file(&self, id: &str, contents: &str) -> io::Result<PathBuf> {
        let dir = self.themes_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{id}.json"));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        io::Write::write_all(&mut file, contents.as_bytes())?;
        Ok(path)
    }

    /// Reads a file in the config directory as UTF-8, refusing oversized files.
    pub fn read_file(path: &Path) -> io::Result<String> {
        let size = fs::metadata(path)?.len();
        if size > MAX_FILE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("file is {size} bytes; the limit is {MAX_FILE_BYTES}"),
            ));
        }
        fs::read_to_string(path)
    }
}

fn files_with_extension(dir: &Path, extension: &str) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == extension) && path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match ConfigDir::read_file(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn platform_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("super-keyboard"))
}

/// The app's private files directory, `/data/user/<user>/<package>/files`.
///
/// Worked out from the process rather than through JNI: Android names app
/// processes after their package, and app uids are `user * 100000 + app id`.
#[cfg(target_os = "android")]
fn platform_dir() -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;

    let cmdline = fs::read("/proc/self/cmdline").ok()?;
    let process = cmdline.split(|b| *b == 0).next()?;
    let process = std::str::from_utf8(process).ok()?;
    // Secondary processes are named `package:name`.
    let package = process.split(':').next()?;
    if package.is_empty() || package.contains('/') {
        return None;
    }
    let user = fs::metadata("/proc/self").ok()?.uid() / 100_000;
    let files = PathBuf::from(format!("/data/user/{user}/{package}/files"));
    files.is_dir().then_some(files)
}

#[cfg(target_arch = "wasm32")]
fn platform_dir() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_inside_the_root() {
        let dir = ConfigDir::new("/home/me/.config/super-keyboard");
        assert!(
            dir.settings_file()
                .ends_with("super-keyboard/settings.toml")
        );
        assert!(dir.layouts_dir().ends_with("super-keyboard/layouts"));
        assert!(dir.themes_dir().ends_with("super-keyboard/themes"));
        let file = dir.layouts_dir().join("mine.toml");
        assert_eq!(
            dir.display_path(&file),
            format!("layouts{}mine.toml", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(dir.display_path(Path::new("/etc/x")), "/etc/x");
    }

    #[test]
    fn missing_files_are_not_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path().join("nothing-here"));
        assert_eq!(dir.read_settings().unwrap(), None);
        assert!(dir.layout_files().unwrap().is_empty());
        assert!(dir.theme_files().unwrap().is_empty());
    }

    #[test]
    fn writes_settings_and_lists_only_toml_layouts() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path().join("cfg"));
        dir.write_settings("theme = \"dark\"\n").unwrap();
        assert_eq!(
            dir.read_settings().unwrap().as_deref(),
            Some("theme = \"dark\"\n")
        );
        assert!(!dir.root().join(".settings.toml.tmp").exists());

        fs::create_dir_all(dir.layouts_dir().join("nested.toml")).unwrap();
        for name in ["b.toml", "a.toml", "notes.txt"] {
            fs::write(dir.layouts_dir().join(name), "").unwrap();
        }
        let names: Vec<_> = dir
            .layout_files()
            .unwrap()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.toml", "b.toml"]);
    }

    #[test]
    fn creates_theme_files_without_overwriting() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = ConfigDir::new(tmp.path().join("cfg"));
        let path = dir.create_theme_file("mine", "{}").unwrap();
        assert_eq!(path, dir.themes_dir().join("mine.json"));
        fs::write(dir.themes_dir().join("notes.txt"), "").unwrap();
        assert_eq!(dir.theme_files().unwrap(), std::slice::from_ref(&path));
        let err = dir.create_theme_file("mine", "new").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(path).unwrap(), "{}");
    }

    #[test]
    fn refuses_oversized_files() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("big.toml");
        fs::write(&path, vec![b'#'; MAX_FILE_BYTES as usize + 1]).unwrap();
        let err = ConfigDir::read_file(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
