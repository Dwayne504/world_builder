//! Versioned application-level preferences: default Projects/Backups
//! directories, stored in the OS application-config directory, entirely
//! outside every `.wcproj` package. Preferences are never Project data:
//! changing them never moves an existing Project or backup, they only seed
//! the default location offered for the *next* operation.

pub mod error;
pub mod store;

pub use error::PreferencesError;
pub use store::PreferencesStore;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const PREFERENCES_SCHEMA_VERSION: i64 = 1;
pub const PREFERENCES_FILE: &str = "preferences.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppPreferences {
    // Intentionally has no `#[serde(default)]`: a file missing this field
    // is malformed and must fail as `Corrupt`, never be silently treated
    // as the current schema version.
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

/// Parses `bytes` and checks the schema version before returning success.
/// A version newer than this build supports is refused so it is never
/// silently rewritten by an older build; a version older than the current
/// one is refused too, since no migration from an earlier version has ever
/// been published yet -- this is the documented, intentional behavior
/// until a real migration exists, rather than an accidental gap.
fn parse_and_check_version(bytes: &[u8]) -> Result<AppPreferences, PreferencesError> {
    let parsed: AppPreferences =
        serde_json::from_slice(bytes).map_err(|e| PreferencesError::Corrupt(e.to_string()))?;
    if parsed.schema_version != PREFERENCES_SCHEMA_VERSION {
        return Err(PreferencesError::UnsupportedVersion {
            found: parsed.schema_version,
            supported: PREFERENCES_SCHEMA_VERSION,
        });
    }
    Ok(parsed)
}

/// Loads preferences from `path`, first repairing an interrupted
/// publication (see `crate::atomic_file`). A missing file means "no
/// preferences configured yet" and returns defaults; a corrupt or
/// unsupported-schema-version file fails safely instead of being silently
/// discarded, downgraded, or rewritten.
pub fn load(path: &Path) -> Result<AppPreferences, PreferencesError> {
    let _ = crate::atomic_file::recover(path, |bytes| parse_and_check_version(bytes).is_ok());
    if !path.exists() {
        return Ok(AppPreferences::default());
    }
    let raw = fs::read(path)?;
    parse_and_check_version(&raw)
}

/// Publishes `prefs` at `path` via the shared Windows-safe, recoverable
/// publish protocol (see `crate::atomic_file`). This does not claim atomic
/// replacement on every platform: `fs::rename` cannot replace an existing
/// file on Windows, so the previous file is moved aside as `.previous`
/// immediately before the validated successor is published, and cleaned
/// up only after publication succeeds.
pub fn save(path: &Path, prefs: &AppPreferences) -> Result<(), PreferencesError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = serde_json::to_string_pretty(prefs)
        .map_err(|e| PreferencesError::Corrupt(e.to_string()))?;
    crate::atomic_file::publish(path, serialized.as_bytes(), |bytes| {
        parse_and_check_version(bytes).is_ok()
    })
    .map_err(PreferencesError::Io)
}

/// Explicit, deliberate recovery from a corrupt or unsupported-version
/// preferences file: preserves the existing file untouched under a unique
/// diagnostic filename (never overwriting earlier diagnostic evidence),
/// then publishes fresh defaults. Never invoked automatically.
pub fn reset(path: &Path) -> Result<AppPreferences, PreferencesError> {
    if path.is_file() {
        let diagnostic = diagnostic_backup_path(path);
        fs::rename(path, &diagnostic)?;
    }
    let defaults = AppPreferences::default();
    save(path, &defaults)?;
    Ok(defaults)
}

fn diagnostic_backup_path(path: &Path) -> PathBuf {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("preferences.json");
    let candidate =
        |suffix: &str| path.with_file_name(format!("{file_name}.corrupt-{stamp}{suffix}"));
    let mut backup = candidate("");
    // Guarantee uniqueness even across repeated resets within the same
    // millisecond (e.g. in tests): never overwrite prior diagnostic
    // evidence.
    let mut disambiguator = 2;
    while backup.exists() {
        backup = candidate(&format!("-{disambiguator}"));
        disambiguator += 1;
    }
    backup
}

/// True only when `path` currently exists and is a directory. Used to
/// detect a configured default directory that has since been moved or
/// become inaccessible, without ever silently falling back to a different
/// location.
pub fn directory_is_usable(path: &Path) -> bool {
    path.is_dir()
}

/// Validates a directory *at the moment it is chosen* for a preference: it
/// must currently exist and be a directory. Distinct from
/// `directory_is_usable`, which only reports the state of an already
/// configured value for display.
pub fn validate_directory(path: &Path) -> Result<(), PreferencesError> {
    if directory_is_usable(path) {
        Ok(())
    } else {
        Err(PreferencesError::InvalidDirectory(
            path.display().to_string(),
        ))
    }
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
    fn save_leaves_no_next_or_previous_file_behind_on_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        save(&path, &AppPreferences::default()).unwrap();
        assert!(!path.with_extension("json.next").exists());
        assert!(!path.with_extension("json.previous").exists());
    }

    #[test]
    fn a_later_save_replaces_the_file_and_leaves_no_leftovers() {
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
        assert!(!path.with_extension("json.next").exists());
        assert!(!path.with_extension("json.previous").exists());
    }

    #[test]
    fn interrupted_publication_with_only_a_successor_present_recovers_it() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let prefs = AppPreferences {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            default_projects_dir: Some(PathBuf::from("/recovered")),
            default_backups_dir: None,
        };
        // Simulate a crash after the successor was written+synced but
        // before it was renamed into place: only `.next` exists.
        fs::write(
            path.with_extension("json.next"),
            serde_json::to_string_pretty(&prefs).unwrap(),
        )
        .unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded, prefs);
        assert!(path.is_file());
        assert!(!path.with_extension("json.next").exists());
    }

    #[test]
    fn interrupted_publication_with_previous_and_missing_destination_recovers_previous() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let prior = AppPreferences {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            default_projects_dir: Some(PathBuf::from("/prior")),
            default_backups_dir: None,
        };
        // Simulate a crash after the prior file was moved aside as
        // `.previous` but before the successor was renamed into place.
        fs::write(
            path.with_extension("json.previous"),
            serde_json::to_string_pretty(&prior).unwrap(),
        )
        .unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded, prior);
    }

    #[test]
    fn interrupted_publication_with_both_previous_and_valid_current_keeps_current() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let current = AppPreferences {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            default_projects_dir: Some(PathBuf::from("/current")),
            default_backups_dir: None,
        };
        save(&path, &current).unwrap();
        // A leftover `.previous` from an earlier, already-completed
        // publish must never override an already-valid destination.
        fs::write(path.with_extension("json.previous"), b"stale").unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded, current);
        assert_eq!(
            fs::read(path.with_extension("json.previous")).unwrap(),
            b"stale"
        );
    }

    #[test]
    fn recovery_selects_a_valid_complete_file_over_a_corrupt_next() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let prior = AppPreferences {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            default_projects_dir: Some(PathBuf::from("/still-valid")),
            default_backups_dir: None,
        };
        fs::write(path.with_extension("json.next"), b"{ truncated").unwrap();
        fs::write(
            path.with_extension("json.previous"),
            serde_json::to_string_pretty(&prior).unwrap(),
        )
        .unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded, prior);
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
    fn missing_schema_version_field_is_corrupt_not_silently_the_current_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::write(
            &path,
            br#"{"default_projects_dir":null,"default_backups_dir":null}"#,
        )
        .unwrap();
        assert!(matches!(load(&path), Err(PreferencesError::Corrupt(_))));
    }

    #[test]
    fn a_newer_unsupported_schema_version_is_rejected_and_never_rewritten() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let bytes_before =
            br#"{"schema_version":99,"default_projects_dir":null,"default_backups_dir":null}"#;
        fs::write(&path, bytes_before).unwrap();
        let err = load(&path).unwrap_err();
        assert!(matches!(
            err,
            PreferencesError::UnsupportedVersion {
                found: 99,
                supported: PREFERENCES_SCHEMA_VERSION
            }
        ));
        assert_eq!(fs::read(&path).unwrap(), bytes_before);
    }

    #[test]
    fn an_older_schema_version_is_also_rejected_until_a_migration_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::write(
            &path,
            br#"{"schema_version":0,"default_projects_dir":null,"default_backups_dir":null}"#,
        )
        .unwrap();
        assert!(matches!(
            load(&path),
            Err(PreferencesError::UnsupportedVersion { found: 0, .. })
        ));
    }

    #[test]
    fn reset_preserves_the_corrupt_file_under_a_diagnostic_name_then_writes_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::write(&path, b"{ not valid json").unwrap();

        let reset_result = reset(&path).unwrap();
        assert_eq!(reset_result, AppPreferences::default());
        assert_eq!(load(&path).unwrap(), AppPreferences::default());

        // The corrupt evidence was preserved, not overwritten or deleted.
        let siblings: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        let diagnostic = siblings
            .iter()
            .find(|name| name.contains(".corrupt-"))
            .expect("a diagnostic backup file must exist");
        assert_eq!(
            fs::read(dir.path().join(diagnostic)).unwrap(),
            b"{ not valid json"
        );
    }

    #[test]
    fn resetting_twice_never_overwrites_the_first_diagnostic_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::write(&path, b"first corrupt copy").unwrap();
        reset(&path).unwrap();
        fs::write(&path, b"{ not valid json").unwrap();
        reset(&path).unwrap();

        let diagnostics: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".corrupt-"))
            .collect();
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn missing_or_moved_configured_directory_is_reported_as_unusable() {
        let dir = tempdir().unwrap();
        let moved_away = dir.path().join("no-longer-here");
        assert!(!directory_is_usable(&moved_away));
        assert!(directory_is_usable(dir.path()));
    }

    #[test]
    fn validate_directory_rejects_a_path_that_is_not_an_existing_directory() {
        let dir = tempdir().unwrap();
        assert!(validate_directory(dir.path()).is_ok());
        let missing = dir.path().join("nope");
        assert!(matches!(
            validate_directory(&missing),
            Err(PreferencesError::InvalidDirectory(_))
        ));
        let file_path = dir.path().join("a-file");
        fs::write(&file_path, b"x").unwrap();
        assert!(matches!(
            validate_directory(&file_path),
            Err(PreferencesError::InvalidDirectory(_))
        ));
    }
}
