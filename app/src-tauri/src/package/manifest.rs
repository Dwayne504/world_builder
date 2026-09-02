//! `manifest.json`: the portable package identity authority.
//!
//! `manifest.json`'s `project_id` must equal the database's
//! `project_meta.project_id`; a mismatch is rejected, never silently
//! repaired (see `persistence::worker`). `working_name` here is a
//! *non-authoritative cache* for quick display without opening SQLite --
//! the database row is the source of truth and is refreshed on every
//! successful rename.

use std::fs;
use std::io::Write;
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
        recover(path)?;
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(|e| PackageError::InvalidManifest(e.to_string()))
    }

    /// Writes the manifest atomically: write to a sibling temp file, then
    /// rename over the destination, so a crash mid-write never leaves a
    /// truncated/corrupt `manifest.json`.
    pub fn write(&self, path: &Path) -> Result<(), PackageError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| PackageError::InvalidManifest(e.to_string()))?;
        let tmp_path = path.with_extension("json.next");
        let previous_path = path.with_extension("json.previous");
        {
            let mut f = fs::File::create(&tmp_path)?;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
        }
        if let Err(error) = fs::rename(&tmp_path, path) {
            // Windows cannot replace an existing file with rename. Preserve
            // the prior copy under a deterministic recovery name before
            // publishing the synced successor; `read` repairs an interruption.
            if path.exists() {
                let _ = fs::remove_file(&previous_path);
                fs::rename(path, &previous_path)?;
                if let Err(rename_error) = fs::rename(&tmp_path, path) {
                    return Err(PackageError::Io(rename_error));
                }
            } else {
                let _ = fs::remove_file(&tmp_path);
                return Err(PackageError::Io(error));
            }
        }
        if let Some(parent) = path.parent() {
            if let Ok(dir) = fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }
        Ok(())
    }
}

fn recover(path: &Path) -> Result<(), PackageError> {
    if path.is_file() {
        return Ok(());
    }
    let next = path.with_extension("json.next");
    let previous = path.with_extension("json.previous");
    for candidate in [&next, &previous] {
        if candidate.is_file()
            && fs::read_to_string(candidate)
                .ok()
                .and_then(|raw| serde_json::from_str::<Manifest>(&raw).ok())
                .is_some()
        {
            fs::rename(candidate, path)?;
            return Ok(());
        }
    }
    Ok(())
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
}
