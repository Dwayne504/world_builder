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

/// Sanitizes a working name into a filesystem-safe (but non-authoritative)
/// directory stem. This never becomes identity: it only seeds the initial
/// directory name shown to the user at creation time.
pub fn sanitize_directory_stem(working_name: &str) -> String {
    let mut stem: String = working_name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string();
    if stem.is_empty() {
        stem = "Untitled Project".to_string();
    }
    stem
}

/// Builds an available (non-colliding) package path under `base_dir` for
/// the given working name, trying `Name.wcproj`, `Name (2).wcproj`, etc.
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

/// Validates that `root` looks like a Worldcrafter package (has a manifest)
/// without yet parsing or trusting its contents.
pub fn validate_structure(root: &Path) -> Result<PackagePaths, PackageError> {
    let paths = PackagePaths::new(root);
    if !paths.manifest_path().is_file() {
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
}
