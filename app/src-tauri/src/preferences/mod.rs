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

pub const PREFERENCES_SCHEMA_VERSION: i64 = 2;
pub const PREFERENCES_FILE: &str = "preferences.json";

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    Storybook,
    Starship,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppPreferences {
    // Intentionally has no `#[serde(default)]`: a file missing this field
    // is malformed and must fail as `Corrupt`, never be silently treated
    // as the current schema version.
    pub schema_version: i64,
    pub appearance: Appearance,
    pub default_projects_dir: Option<PathBuf>,
    pub default_backups_dir: Option<PathBuf>,
}

impl Default for AppPreferences {
    fn default() -> Self {
        AppPreferences {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            appearance: Appearance::Storybook,
            default_projects_dir: None,
            default_backups_dir: None,
        }
    }
}

/// Version 1 is migrated in memory; a read never rewrites a valid file.
/// The next explicit preference update publishes version 2 through the same
/// recoverable protocol. Unknown versions and invalid appearances fail closed.
fn parse_and_check_version(bytes: &[u8]) -> Result<AppPreferences, PreferencesError> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| PreferencesError::Corrupt(e.to_string()))?;
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| PreferencesError::Corrupt("Missing or invalid schema version".into()))?;
    match version {
        1 => {
            value["schema_version"] = PREFERENCES_SCHEMA_VERSION.into();
            value["appearance"] = serde_json::to_value(Appearance::default()).unwrap();
        }
        PREFERENCES_SCHEMA_VERSION => {}
        found => {
            return Err(PreferencesError::UnsupportedVersion {
                found,
                supported: PREFERENCES_SCHEMA_VERSION,
            })
        }
    }
    serde_json::from_value(value).map_err(|e| PreferencesError::Corrupt(e.to_string()))
}

/// Loads preferences from `path`, first repairing an interrupted
/// publication (see `crate::atomic_file`). A missing file means "no
/// preferences configured yet" and returns defaults; a corrupt or
/// unsupported-schema-version file fails safely instead of being silently
/// discarded, downgraded, or rewritten.
pub fn load(path: &Path) -> Result<AppPreferences, PreferencesError> {
    if let Some(raw) = read_optional(path)? {
        return parse_and_check_version(&raw);
    }
    let mut valid = None;
    let mut corrupt = None;
    for candidate in [
        crate::atomic_file::next_path(path),
        crate::atomic_file::previous_path(path),
    ] {
        if let Some(raw) = read_optional(&candidate)? {
            match parse_and_check_version(&raw) {
                Ok(prefs) => {
                    if valid.is_none() {
                        valid = Some((candidate, prefs));
                    }
                }
                Err(error @ PreferencesError::UnsupportedVersion { .. }) => return Err(error),
                Err(error) => corrupt = Some(error),
            }
        }
    }
    if let Some((candidate, prefs)) = valid {
        fs::rename(candidate, path)?;
        return Ok(prefs);
    }
    match corrupt {
        Some(error) => Err(error),
        None => Ok(AppPreferences::default()),
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, PreferencesError> {
    match fs::read(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Publishes `prefs` at `path` via the shared Windows-safe, recoverable
/// publish protocol (see `crate::atomic_file`). This does not claim atomic
/// replacement on every filesystem: if direct replacement fails, the old
/// file is moved aside as `.previous` before the validated successor is
/// published, and cleaned up only after publication succeeds.
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

/// Explicit, deliberate recovery from corrupt
/// preferences file: preserves the existing file untouched under a unique
/// diagnostic filename (never overwriting earlier diagnostic evidence),
/// then publishes fresh defaults. Never invoked automatically.
pub fn reset(path: &Path) -> Result<AppPreferences, PreferencesError> {
    let candidates = [
        path.to_path_buf(),
        crate::atomic_file::next_path(path),
        crate::atomic_file::previous_path(path),
    ];
    // A newer schema may have changed all other fields: never downgrade it,
    // even via reset, and never lose interrupted-publication evidence.
    for candidate in &candidates {
        if let Some(raw) = read_optional(candidate)? {
            if let Err(error @ PreferencesError::UnsupportedVersion { .. }) =
                parse_and_check_version(&raw)
            {
                return Err(error);
            }
        }
    }
    for candidate in &candidates {
        if candidate.is_file() {
            fs::rename(candidate, diagnostic_backup_path(candidate))?;
        }
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
    validate_directory(path).is_ok()
}

/// Validates a directory *at the moment it is chosen* for a preference: it
/// must currently exist and be a directory. Distinct from
/// `directory_is_usable`, which only reports the state of an already
/// configured value for display.
pub fn validate_directory(path: &Path) -> Result<(), PreferencesError> {
    let invalid = || PreferencesError::InvalidDirectory(path.display().to_string());
    if !path.is_absolute() || !path.is_dir() {
        return Err(invalid());
    }
    let canonical = fs::canonicalize(path).map_err(|_| invalid())?;
    if path
        .ancestors()
        .chain(canonical.ancestors())
        .any(|ancestor| {
            ancestor
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("wcproj") || ext.eq_ignore_ascii_case("wcbackup")
                })
        })
    {
        return Err(invalid());
    }
    fs::read_dir(&canonical).map_err(|_| invalid())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn version_one_migrates_without_writing_and_keeps_directories_on_appearance_update() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let bytes = br#"{"schema_version":1,"default_projects_dir":"/Projects","default_backups_dir":"/Backups"}"#;
        fs::write(&path, bytes).unwrap();
        let store = PreferencesStore::new(&path);
        let read = store.load().unwrap();
        assert_eq!(read.appearance, Appearance::Storybook);
        assert_eq!(read.schema_version, 2);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        store
            .update(|p| p.appearance = Appearance::Starship)
            .unwrap();
        let restarted = PreferencesStore::new(&path).load().unwrap();
        assert_eq!(restarted.appearance, Appearance::Starship);
        assert_eq!(restarted.default_projects_dir, read.default_projects_dir);
        assert_eq!(restarted.default_backups_dir, read.default_backups_dir);
    }

    #[test]
    fn invalid_appearance_is_visible_and_never_overwritten() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let bytes = br#"{"schema_version":2,"appearance":"unknown","default_projects_dir":null,"default_backups_dir":null}"#;
        fs::write(&path, bytes).unwrap();
        let store = PreferencesStore::new(&path);
        assert!(matches!(store.load(), Err(PreferencesError::Corrupt(_))));
        assert!(store
            .update(|p| p.appearance = Appearance::Storybook)
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn newer_shapes_and_interrupted_publications_fail_closed() {
        for suffix in ["json", "json.next", "json.previous"] {
            let dir = tempdir().unwrap();
            let path = dir.path().join("preferences.json");
            let candidate = path.with_extension(suffix);
            let future = br#"{"schema_version":99,"default_projects_dir":{"new_shape":true}}"#;
            fs::write(&candidate, future).unwrap();
            assert!(matches!(
                load(&path),
                Err(PreferencesError::UnsupportedVersion { .. })
            ));
            assert!(matches!(
                reset(&path),
                Err(PreferencesError::UnsupportedVersion { .. })
            ));
            assert_eq!(fs::read(candidate).unwrap(), future);
        }
    }

    #[test]
    fn corrupt_recovery_files_are_visible_and_reset_preserves_all_evidence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::write(crate::atomic_file::next_path(&path), b"broken next").unwrap();
        fs::write(crate::atomic_file::previous_path(&path), b"broken previous").unwrap();
        assert!(matches!(load(&path), Err(PreferencesError::Corrupt(_))));
        reset(&path).unwrap();
        let evidence: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|p| p.to_string_lossy().contains(".corrupt-"))
            .map(|p| fs::read(p).unwrap())
            .collect();
        assert!(evidence.contains(&b"broken next".to_vec()));
        assert!(evidence.contains(&b"broken previous".to_vec()));
        assert_eq!(load(&path).unwrap(), AppPreferences::default());
    }

    #[test]
    fn recovery_io_errors_never_become_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::create_dir(crate::atomic_file::next_path(&path)).unwrap();
        assert!(matches!(load(&path), Err(PreferencesError::Io(_))));
        assert!(!path.exists());
    }

    #[test]
    fn valid_recovery_copy_never_downgrades_an_unsupported_sibling() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let next = crate::atomic_file::next_path(&path);
        let previous = crate::atomic_file::previous_path(&path);
        fs::write(
            &next,
            serde_json::to_vec(&AppPreferences::default()).unwrap(),
        )
        .unwrap();
        fs::write(&previous, br#"{"schema_version":99}"#).unwrap();
        assert!(matches!(
            load(&path),
            Err(PreferencesError::UnsupportedVersion { .. })
        ));
        assert!(!path.exists());
        assert!(next.is_file() && previous.is_file());
    }

    #[test]
    fn directories_must_be_absolute_and_outside_packages() {
        assert!(validate_directory(Path::new(".")).is_err());
        let dir = tempdir().unwrap();
        for package in ["Example.wcproj", "Example.WCBACKUP"] {
            let nested = dir.path().join(package).join("assets");
            fs::create_dir_all(&nested).unwrap();
            assert!(validate_directory(&nested).is_err());
            assert!(validate_directory(nested.parent().unwrap()).is_err());
        }
        assert!(validate_directory(dir.path()).is_ok());
    }

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
            appearance: Appearance::Storybook,
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
                appearance: Appearance::Storybook,
                default_projects_dir: Some(PathBuf::from("/old/projects")),
                default_backups_dir: None,
            },
        )
        .unwrap();
        save(
            &path,
            &AppPreferences {
                schema_version: PREFERENCES_SCHEMA_VERSION,
                appearance: Appearance::Storybook,
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
            appearance: Appearance::Storybook,
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
            appearance: Appearance::Storybook,
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
            appearance: Appearance::Storybook,
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
            appearance: Appearance::Storybook,
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
