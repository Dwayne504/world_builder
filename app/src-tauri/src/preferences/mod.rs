//! Versioned application-level preferences: default Projects/Backups
//! directories, stored in the OS application-config directory, entirely
//! outside every `.wcproj` package. Preferences are never Project data:
//! changing them never moves an existing Project or backup, they only seed
//! the default location offered for the *next* operation.

pub mod error;

pub use error::PreferencesError;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PREFERENCES_SCHEMA_VERSION: i64 = 1;
pub const PREFERENCES_FILE: &str = "preferences.json";

fn default_schema_version() -> i64 {
    PREFERENCES_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppPreferences {
    #[serde(default = "default_schema_version")]
    pub schema_version: i64,
    pub default_projects_dir: Option<PathBuf>,
    pub default_backups_dir: Option<PathBuf>,
}

impl Default for AppPreferences {
    fn default() -> Self {
        AppPreferences {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            default_projects_dir: None,
            default_backups_dir: None,
        }
    }
}

/// Loads preferences from `path`. A missing file means "no preferences
/// configured yet" and returns defaults; a present-but-corrupt file fails
/// safely instead of silently discarding it, so the caller can decide how
/// to surface the problem without ever overwriting the evidence.
pub fn load(path: &Path) -> Result<AppPreferences, PreferencesError> {
    if !path.exists() {
        return Ok(AppPreferences::default());
    }
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|e| PreferencesError::Corrupt(e.to_string()))
}

/// Atomically replaces the preferences file: write-to-temp, `fsync`,
/// rename-into-place. A crash mid-write can never leave a half-written
/// preferences file at `path`, and a reader never observes a partial write.
pub fn save(path: &Path, prefs: &AppPreferences) -> Result<(), PreferencesError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("json.{}.tmp", Uuid::new_v4()));
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let serialized = serde_json::to_string_pretty(prefs)
            .map_err(|e| PreferencesError::Corrupt(e.to_string()))?;
        file.write_all(serialized.as_bytes())?;
        file.sync_all()?;
    }
    // Atomic on both POSIX and Windows (ReplaceFile-equivalent rename).
    if let Err(e) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(e.into());
    }
    Ok(())
}

/// True only when `path` currently exists and is a directory. Used to
/// detect a configured default directory that has since been moved or
/// become inaccessible, without ever silently falling back to a different
/// location.
pub fn directory_is_usable(path: &Path) -> bool {
    path.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_loads_as_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        assert_eq!(load(&path).unwrap(), AppPreferences::default());
    }

    #[test]
    fn round_trips_across_a_simulated_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let prefs = AppPreferences {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            default_projects_dir: Some(dir.path().join("Projects")),
            default_backups_dir: Some(dir.path().join("Backups")),
        };
        save(&path, &prefs).unwrap();
        // A fresh load simulates a new process reading the persisted file.
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded, prefs);
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temp_file_behind() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        save(&path, &AppPreferences::default()).unwrap();
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn a_later_save_replaces_the_file_without_ever_removing_it_first() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        save(
            &path,
            &AppPreferences {
                schema_version: PREFERENCES_SCHEMA_VERSION,
                default_projects_dir: Some(PathBuf::from("/old/projects")),
                default_backups_dir: None,
            },
        )
        .unwrap();
        save(
            &path,
            &AppPreferences {
                schema_version: PREFERENCES_SCHEMA_VERSION,
                default_projects_dir: Some(PathBuf::from("/new/projects")),
                default_backups_dir: None,
            },
        )
        .unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(
            reloaded.default_projects_dir,
            Some(PathBuf::from("/new/projects"))
        );
    }

    #[test]
    fn corrupt_preferences_fail_safely_and_are_never_deleted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::write(&path, b"{ not valid json").unwrap();
        assert!(matches!(load(&path), Err(PreferencesError::Corrupt(_))));
        // The unreadable file is preserved for inspection, not swept away.
        assert_eq!(fs::read(&path).unwrap(), b"{ not valid json");
    }

    #[test]
    fn missing_or_moved_configured_directory_is_reported_as_unusable() {
        let dir = tempdir().unwrap();
        let moved_away = dir.path().join("no-longer-here");
        assert!(!directory_is_usable(&moved_away));
        assert!(directory_is_usable(dir.path()));
    }
}
