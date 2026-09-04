//! `manifest.json`: the portable package identity authority.
//!
//! `manifest.json`'s `project_id` must equal the database's
//! `project_meta.project_id`; a mismatch is rejected, never silently
//! repaired (see `persistence::worker`). `working_name` here is a
//! *non-authoritative cache* for quick display without opening SQLite --
//! the database row is the source of truth and is refreshed on every
//! successful rename.

use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ProjectId;

use super::error::PackageError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub project_id: ProjectId,
    pub format_version: i64,
    pub schema_version: i64,
    pub created_at: DateTime<Utc>,
    /// Non-authoritative display-name cache; see module docs.
    pub working_name_cache: String,
    pub restored_from_project_id: Option<ProjectId>,
    pub restored_from_backup_id: Option<String>,
}

impl Manifest {
    pub fn new(
        project_id: ProjectId,
        format_version: i64,
        schema_version: i64,
        working_name: &str,
    ) -> Self {
        Manifest {
            project_id,
            format_version,
            schema_version,
            created_at: Utc::now(),
            working_name_cache: working_name.to_string(),
            restored_from_project_id: None,
            restored_from_backup_id: None,
        }
    }

    pub fn read(path: &Path) -> Result<Self, PackageError> {
        Self::recover_if_needed(path)?;
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(|e| PackageError::InvalidManifest(e.to_string()))
    }

    /// Repairs an interrupted atomic manifest replacement if a valid synced
    /// sibling recovery file exists. This is safe to call before package
    /// structure checks so an open/validate path can recover a missing
    /// `manifest.json` instead of rejecting the package outright.
    pub fn recover_if_needed(path: &Path) -> Result<(), PackageError> {
        recover(path)
    }

    /// Writes the manifest via the shared Windows-safe, recoverable
    /// publish protocol (see `crate::atomic_file`): a crash mid-write or
    /// mid-publish never leaves a truncated/corrupt `manifest.json`, and
    /// `read`/`recover_if_needed` repair an interruption on next open.
    pub fn write(&self, path: &Path) -> Result<(), PackageError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| PackageError::InvalidManifest(e.to_string()))?;
        crate::atomic_file::publish(path, json.as_bytes(), |bytes| {
            serde_json::from_slice::<Manifest>(bytes).is_ok()
        })
        .map_err(PackageError::Io)
    }
}

fn recover(path: &Path) -> Result<(), PackageError> {
    crate::atomic_file::recover(path, |bytes| {
        serde_json::from_slice::<Manifest>(bytes).is_ok()
    })
    .map_err(PackageError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trips_through_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        let manifest = Manifest::new(ProjectId::new(), 1, 1, "Tortuga");
        manifest.write(&path).unwrap();
        let read_back = Manifest::read(&path).unwrap();
        assert_eq!(manifest, read_back);
    }

    #[test]
    fn rejects_corrupt_manifest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        fs::write(&path, "{ not json").unwrap();
        assert!(Manifest::read(&path).is_err());
    }

    #[test]
    fn repeatedly_replaces_an_existing_manifest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        let first = Manifest::new(ProjectId::new(), 1, 1, "Tortuga");
        first.write(&path).unwrap();
        let second = Manifest::new(ProjectId::new(), 1, 1, "Arak");
        second.write(&path).unwrap();
        assert_eq!(Manifest::read(&path).unwrap(), second);
    }

    #[test]
    fn read_recovers_a_synced_successor_or_prior_manifest_after_interruption() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        let manifest = Manifest::new(ProjectId::new(), 1, 1, "Tortuga");
        fs::write(
            path.with_extension("json.next"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(Manifest::read(&path).unwrap(), manifest);

        fs::remove_file(&path).unwrap();
        fs::write(
            path.with_extension("json.previous"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(Manifest::read(&path).unwrap(), manifest);
    }

    #[test]
    fn explicit_recovery_is_safe_when_manifest_already_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        let manifest = Manifest::new(ProjectId::new(), 1, 1, "Tortuga");
        manifest.write(&path).unwrap();

        Manifest::recover_if_needed(&path).unwrap();

        assert_eq!(Manifest::read(&path).unwrap(), manifest);
    }
}
