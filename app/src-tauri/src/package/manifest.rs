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
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(|e| PackageError::InvalidManifest(e.to_string()))
    }

    /// Writes the manifest atomically: write to a sibling temp file, then
    /// rename over the destination, so a crash mid-write never leaves a
    /// truncated/corrupt `manifest.json`.
    pub fn write(&self, path: &Path) -> Result<(), PackageError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| PackageError::InvalidManifest(e.to_string()))?;
        let tmp_path = path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp_path)?;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp_path, path)?;
        Ok(())
    }
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
}
