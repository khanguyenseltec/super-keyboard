//! Errors from reading or editing `settings.toml`.

use super::Limits;

/// Why `settings.toml` could not be used.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SettingsError {
    /// Not valid TOML, an unknown field or a value of the wrong type. The message
    /// includes the line and column.
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    /// Not valid TOML, found while editing the file.
    #[error("{0}")]
    Edit(#[from] toml_edit::TomlError),
    #[error(
        "`{field}` must be {}a theme's name (letters, digits, `-` and `_`), not {name:?}",
        if *field == "theme" { "\"system\" or " } else { "" }
    )]
    InvalidThemeName { field: &'static str, name: String },
    #[error("`profile` must not be empty")]
    EmptyProfile,
    #[error("profile name `{0}` may only use letters, digits, `-` and `_`")]
    InvalidProfileName(String),
    #[error("profile `{0}` is defined more than once")]
    DuplicateProfile(String),
    #[error("profile `{0}` needs a `layout`")]
    EmptyProfileLayout(String),
    #[error("`{field}` must be between {} and {}, not {value}", limits.min, limits.max)]
    OutOfRange {
        field: &'static str,
        value: i64,
        limits: Limits,
    },
    #[error(
        "`{field}` must be \"auto\" or {}",
        if field.starts_with("keys.") { "a number of pixels" } else { "a percentage of the window" }
    )]
    NotAutoOrNumber { field: &'static str },
    #[error("`keyboard.align` must be \"center\", \"left\" or \"right\", not {0:?}")]
    UnknownAlign(String),
    #[error("`vietnamese.method` must be \"telex\" or \"vni\", not {0:?}")]
    UnknownInputMethod(String),
    #[error("`vietnamese.tone-style` must be \"classic\" or \"modern\", not {0:?}")]
    UnknownToneStyle(String),
    #[error("unknown style variable `{path}` (known: {known})")]
    UnknownStyleVar { path: String, known: String },
    #[error("`{path}`: {reason}")]
    InvalidStyleValue { path: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeySettings;

    #[test]
    fn range_errors_name_the_field_and_limits() {
        let err = SettingsError::OutOfRange {
            field: "keys.hold-delay",
            value: 5000,
            limits: KeySettings::HOLD_DELAY_MS,
        };
        assert_eq!(
            err.to_string(),
            "`keys.hold-delay` must be between 150 and 1000, not 5000"
        );
    }
}
