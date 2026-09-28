//! Concrete package directory layout and safe (atomic-ish) creation.

use std::fs;
use std::path::{Path, PathBuf};

use super::error::PackageError;

pub const PACKAGE_EXTENSION: &str = "wcproj";
pub const MANIFEST_FILE: &str = "manifest.json";
pub const DATA_DIR: &str = "data";
pub const DB_FILE: &str = "project.sqlite";
pub const ASSETS_DIR: &str = "assets";
pub const STAGING_DIR: &str = "staging";
pub const LOCK_FILE: &str = "lock.json";

/// Resolved paths inside an (already created or opened) package. All paths
/// are computed from `root`; nothing here is machine-specific beyond the
/// root itself, and nothing inside the package stores an absolute path.
#[derive(Debug, Clone)]
pub struct PackagePaths {
    pub root: PathBuf,
}

impl PackagePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        PackagePaths { root: root.into() }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join(DATA_DIR)
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir().join(DB_FILE)
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join(ASSETS_DIR)
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.root.join(STAGING_DIR)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(LOCK_FILE)
    }
}

/// Windows reserved device names (case-insensitive, and reserved even with
/// a trailing extension, e.g. `CON.txt`). Irrelevant on other platforms,
/// but sanitization must produce one filesystem-safe name that works
/// everywhere Worldcrafter runs, not a platform-specific one.
const RESERVED_WINDOWS_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "COM¹", "COM²",
    "COM³", "LPT¹", "LPT²", "LPT³",
];

/// Bounds component bytes, leaving space for package and staging suffixes.
/// Full-path limits still depend on the chosen parent directory.
const MAX_STEM_CHARS: usize = 100;

/// Sanitizes a working name into a filesystem-safe (but non-authoritative)
/// directory stem. This never becomes identity: it only seeds the initial
/// directory name shown to the user at creation time. Handles control
/// characters, characters invalid on supported platforms, Windows reserved
/// device names, trailing spaces/dots (invalid on Windows), and bounds the
/// result to a reasonable length.
pub fn sanitize_directory_stem(working_name: &str) -> String {
    let mut stem: String = working_name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() {
                '_'
            } else if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Windows forbids a trailing space or dot on a file/directory name.
    stem = trim_trailing_space_or_dot(stem.trim());

    if stem.len() > MAX_STEM_CHARS {
        let mut bytes = 0;
        stem = stem
            .chars()
            .take_while(|c| {
                bytes += c.len_utf8();
                bytes <= MAX_STEM_CHARS
            })
            .collect();
        stem = trim_trailing_space_or_dot(stem.trim());
    }

    if stem.is_empty() {
        stem = "Untitled Project".to_string();
    }

    if is_reserved_windows_stem(&stem) {
        // A leading underscore before the base and any extension avoids a
        // reserved device name while keeping the name recognizable.
        stem.insert(0, '_');
    }

    stem
}

fn trim_trailing_space_or_dot(stem: &str) -> String {
    stem.trim_end_matches([' ', '.']).to_string()
}

fn is_reserved_windows_stem(stem: &str) -> bool {
    // The reservation applies to the name before any extension, e.g.
    // `CON.txt` is reserved even though `CON` alone has no extension here.
    let base = stem.split('.').next().unwrap_or(stem).trim_end();
    RESERVED_WINDOWS_STEMS
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(base))
}

/// Builds an available (non-colliding) package path under `base_dir` for
/// the given working name, trying `Name.wcproj`, `Name (2).wcproj`, etc.
/// Used only where silently choosing a nearby free name is appropriate
/// (e.g. Restore as Copy); Project creation uses
/// [`single_candidate_package_path`] instead so a collision is always
/// visibly reported rather than silently side-stepped.
pub fn available_package_path(base_dir: &Path, working_name: &str) -> PathBuf {
    let stem = sanitize_directory_stem(working_name);
    let mut candidate = base_dir.join(format!("{stem}.{PACKAGE_EXTENSION}"));
    let mut suffix = 2;
    while candidate.exists() {
        candidate = base_dir.join(format!("{stem} ({suffix}).{PACKAGE_EXTENSION}"));
        suffix += 1;
    }
    candidate
}

/// Builds the single, filesystem-safe candidate package path under
/// `base_dir` for the given working name (`<working name>.wcproj`), never
/// silently choosing an alternative. Callers must report a collision at
/// this exact path to the user instead of renaming around it.
pub fn single_candidate_package_path(base_dir: &Path, working_name: &str) -> PathBuf {
    let stem = sanitize_directory_stem(working_name);
    base_dir.join(format!("{stem}.{PACKAGE_EXTENSION}"))
}

/// Creates the package's directory skeleton (`data/`, `assets/`,
/// `staging/`) safely: the skeleton is built in a temporary sibling
/// directory and only renamed into place once every directory has been
/// created successfully, so a mid-creation failure never leaves a partial
/// package at `root`.
pub fn create_skeleton(root: &Path) -> Result<PackagePaths, PackageError> {
    if root.exists() {
        return Err(PackageError::AlreadyExists(root.display().to_string()));
    }
    let parent = root
        .parent()
        .ok_or_else(|| PackageError::AlreadyExists(root.display().to_string()))?;
    fs::create_dir_all(parent)?;

    let staging_root = parent.join(format!(
        ".{}.creating-{}",
        root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project"),
        uuid::Uuid::new_v4()
    ));

    let build = || -> Result<(), PackageError> {
        fs::create_dir_all(staging_root.join(DATA_DIR))?;
        fs::create_dir_all(staging_root.join(ASSETS_DIR))?;
        fs::create_dir_all(staging_root.join(STAGING_DIR))?;
        Ok(())
    };

    if let Err(e) = build() {
        let _ = fs::remove_dir_all(&staging_root);
        return Err(e);
    }

    if let Err(e) = fs::rename(&staging_root, root) {
        let _ = fs::remove_dir_all(&staging_root);
        return Err(PackageError::Io(e));
    }

    Ok(PackagePaths::new(root))
}

/// Validates that `root` looks like a Worldcrafter package by ensuring a
/// recoverable manifest and an existing database file are present, without yet
/// trusting the manifest's parsed contents.
pub fn validate_structure(root: &Path) -> Result<PackagePaths, PackageError> {
    let paths = PackagePaths::new(root);
    super::manifest::Manifest::recover_if_needed(&paths.manifest_path())?;
    if !paths.manifest_path().is_file() || !paths.db_path().is_file() {
        return Err(PackageError::NotAPackage(root.display().to_string()));
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sanitizes_unsafe_characters() {
        assert_eq!(sanitize_directory_stem("Tortuga / Isle"), "Tortuga _ Isle");
    }

    #[test]
    fn sanitizes_control_characters() {
        // A leading/trailing control character is whitespace-like enough
        // to be trimmed by the initial `trim()`; embed it mid-string to
        // prove it is still replaced, not merely trimmed away.
        assert_eq!(sanitize_directory_stem("Tor\u{0007}tuga"), "Tor_tuga");
    }

    #[test]
    fn suffixes_windows_reserved_device_names_case_insensitively() {
        for reserved in ["CON", "con", "PRN", "Aux", "NUL", "COM1", "lpt9"] {
            let stem = sanitize_directory_stem(reserved);
            assert_ne!(
                stem.to_ascii_uppercase(),
                reserved.to_ascii_uppercase(),
                "{reserved} must not sanitize to a reserved device name"
            );
        }
        // An unrelated name is left alone.
        assert_eq!(sanitize_directory_stem("Constantine"), "Constantine");
    }

    #[test]
    fn reserved_device_name_is_still_reserved_with_a_trailing_extension_like_stem() {
        for name in ["CON.important", "nul.txt", "COM¹", "LPT².log", "CON .txt"] {
            let sanitized = sanitize_directory_stem(name);
            assert!(!is_reserved_windows_stem(&sanitized), "{name}: {sanitized}");
            let dir = tempdir().unwrap();
            fs::create_dir(dir.path().join(format!("{sanitized}.wcproj"))).unwrap();
        }
        assert!(!is_reserved_windows_stem("Constantine"));
    }

    #[test]
    fn trims_trailing_spaces_and_dots() {
        assert_eq!(sanitize_directory_stem("Tortuga."), "Tortuga");
        assert_eq!(sanitize_directory_stem("Tortuga   "), "Tortuga");
        assert_eq!(sanitize_directory_stem("Tortuga..."), "Tortuga");
    }

    #[test]
    fn bounds_an_excessively_long_working_name() {
        let long_name = "A".repeat(500);
        let stem = sanitize_directory_stem(&long_name);
        assert!(stem.chars().count() <= MAX_STEM_CHARS);
        assert!(!stem.is_empty());
        assert!(!stem.ends_with(' ') && !stem.ends_with('.'));
        let unicode = sanitize_directory_stem(&"界".repeat(100));
        let dir = tempdir().unwrap();
        create_skeleton(&dir.path().join(format!("{unicode}.wcproj"))).unwrap();
    }

    #[test]
    fn create_skeleton_builds_expected_directories() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Tortuga.wcproj");
        let paths = create_skeleton(&root).unwrap();
        assert!(paths.data_dir().is_dir());
        assert!(paths.assets_dir().is_dir());
        assert!(paths.staging_dir().is_dir());
        // No temporary staging directory left behind.
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn create_skeleton_refuses_existing_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Tortuga.wcproj");
        create_skeleton(&root).unwrap();
        assert!(matches!(
            create_skeleton(&root),
            Err(PackageError::AlreadyExists(_))
        ));
    }

    #[test]
    fn available_package_path_avoids_collisions() {
        let dir = tempdir().unwrap();
        let first = available_package_path(dir.path(), "Tortuga");
        fs::create_dir_all(&first).unwrap();
        let second = available_package_path(dir.path(), "Tortuga");
        assert_ne!(first, second);
        assert!(second.to_string_lossy().contains("(2)"));
    }

    #[test]
    fn single_candidate_package_path_never_avoids_collisions() {
        let dir = tempdir().unwrap();
        let first = single_candidate_package_path(dir.path(), "Tortuga");
        fs::create_dir_all(&first).unwrap();
        // Unlike `available_package_path`, this always returns the same
        // candidate so the caller can visibly report the collision.
        let second = single_candidate_package_path(dir.path(), "Tortuga");
        assert_eq!(first, second);
    }

    #[test]
    fn validate_structure_recovers_a_missing_manifest_from_recovery_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Tortuga.wcproj");
        let paths = create_skeleton(&root).unwrap();
        fs::write(paths.db_path(), b"sqlite placeholder").unwrap();
        let manifest = crate::package::manifest::Manifest::new(
            crate::domain::ProjectId::new(),
            1,
            1,
            "Tortuga",
        );
        fs::write(
            paths.manifest_path().with_extension("json.previous"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let validated = validate_structure(&root).unwrap();
        assert_eq!(validated.manifest_path(), paths.manifest_path());
        assert!(paths.manifest_path().is_file());
    }

    #[test]
    fn validate_structure_rejects_a_package_missing_its_database_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Tortuga.wcproj");
        let paths = create_skeleton(&root).unwrap();
        fs::write(paths.manifest_path(), b"{}").unwrap();

        assert!(matches!(
            validate_structure(&root),
            Err(PackageError::NotAPackage(_))
        ));
    }
}
