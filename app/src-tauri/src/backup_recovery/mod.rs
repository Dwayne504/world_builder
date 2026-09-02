//! Manual backup and Restore-as-Copy foundation.
//!
//! A backup is a consistent SQLite snapshot (via the Online Backup API,
//! run on the Project's own worker thread so it observes the same
//! connection every write goes through) plus the manifest and empty
//! managed directories needed to restore it as an independent package.
//! Backups live outside the live `.wcproj` package and never include
//! operational lock/WAL artifacts.

pub mod error;

pub use error::BackupError;

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::domain::{ProjectId, WorkingName};
use crate::package::{layout, manifest::Manifest, PackagePaths};
use crate::persistence::ProjectDbWorker;

/// Creates a manual backup of the Project owning `worker`/`live_paths`
/// under `backup_root` (outside the live package), returning the path to
/// the created `.wcbackup` directory.
pub fn create_backup(
    worker: &ProjectDbWorker,
    live_paths: &PackagePaths,
    backup_root: &Path,
) -> Result<PathBuf, BackupError> {
    ensure_outside_live_package(backup_root, live_paths)?;
    let project_dir = backup_root.join(
        Manifest::read(&live_paths.manifest_path())?
            .project_id
            .to_string(),
    );
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ").to_string();
    let backup_root_path = project_dir.join(format!("{stamp}.wcbackup"));
    let staging_path = project_dir.join(format!(
        ".{stamp}.wcbackup.creating-{}",
        uuid::Uuid::new_v4()
    ));
    let backup_paths = layout::create_skeleton(&staging_path)?;

    let result = (|| -> Result<(), BackupError> {
        // Consistent snapshot via the Online Backup API, run against the
        // live connection on its own worker thread.
        let snapshot = worker.backup_to(backup_paths.db_path())?;

        // Copy the manifest describing the *source* Project; restore
        // rewrites identity on the destination copy, never here.
        let mut manifest = Manifest::read(&live_paths.manifest_path())?;
        manifest.project_id = snapshot.project_id;
        manifest.working_name_cache = snapshot.working_name;
        manifest.format_version = snapshot.format_version;
        manifest.schema_version = snapshot.schema_version;
        manifest.write(&backup_paths.manifest_path())?;

        copy_dir_contents(&live_paths.assets_dir(), &backup_paths.assets_dir())?;
        // `staging/` is intentionally left empty: staged imports are
        // recoverable-but-incomplete and are not portable authored content.

        validate_backup(&staging_path)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            fs::rename(&staging_path, &backup_root_path)?;
            Ok(backup_root_path)
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&staging_path);
            Err(e)
        }
    }
}

/// Validates a backup directory: structure, manifest readability, and that
/// the snapshotted database passes `PRAGMA integrity_check` and agrees with
/// the manifest on Project ID. Never mutates anything.
pub fn validate_backup(backup_root: &Path) -> Result<Manifest, BackupError> {
    let paths = layout::validate_structure(backup_root)
        .map_err(|_| BackupError::NotABackup(backup_root.display().to_string()))?;
    let manifest = Manifest::read(&paths.manifest_path())?;
    if manifest.format_version > crate::package::FORMAT_VERSION
        || manifest.schema_version > crate::persistence::migrations::CURRENT_SCHEMA_VERSION
    {
        return Err(BackupError::NotABackup(backup_root.display().to_string()));
    }

    if !paths.db_path().is_file() {
        return Err(BackupError::CorruptSnapshot(
            backup_root.display().to_string(),
        ));
    }
    let conn = Connection::open(paths.db_path())?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|_| BackupError::CorruptSnapshot(backup_root.display().to_string()))?;
    if integrity != "ok" {
        return Err(BackupError::CorruptSnapshot(
            backup_root.display().to_string(),
        ));
    }
    let (db_project_id, db_format, db_schema): (String, i64, i64) = conn
        .query_row(
            "SELECT project_id, format_version, schema_version FROM project_meta WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| BackupError::CorruptSnapshot(backup_root.display().to_string()))?;
    let db_project_id = ProjectId::parse(&db_project_id)
        .map_err(|_| BackupError::CorruptSnapshot(backup_root.display().to_string()))?;
    if db_project_id != manifest.project_id {
        return Err(BackupError::IdentityMismatch {
            manifest: manifest.project_id,
            database: db_project_id,
        });
    }
    let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if db_format != manifest.format_version
        || db_schema != manifest.schema_version
        || user_version != db_schema
    {
        return Err(BackupError::CorruptSnapshot(
            backup_root.display().to_string(),
        ));
    }

    Ok(manifest)
}

/// Restores a validated backup as an independently editable Project copy
/// under `destination_dir`, allocating a **new** Project ID. The source
/// backup and the live Project it was taken from are never modified.
/// Returns the path to the newly created (but not yet opened) package.
pub fn restore_as_copy(
    backup_root: &Path,
    destination_dir: &Path,
    new_working_name: Option<&str>,
) -> Result<PathBuf, BackupError> {
    let backup_manifest = validate_backup(backup_root)?;
    let backup_paths = layout::PackagePaths::new(backup_root);

    ensure_restore_destination_safe(backup_root, destination_dir)?;
    let working_name = match new_working_name {
        Some(name) => WorkingName::new(name)?.into_string(),
        None => WorkingName::new(&backup_manifest.working_name_cache)?.into_string(),
    };

    let new_root = layout::available_package_path(destination_dir, &working_name);
    let staging_root = destination_dir.join(format!(".restore-{}.creating", uuid::Uuid::new_v4()));
    let new_paths = layout::create_skeleton(&staging_root)?;

    let result = (|| -> Result<(), BackupError> {
        fs::copy(backup_paths.db_path(), new_paths.db_path())?;
        copy_dir_contents(&backup_paths.assets_dir(), &new_paths.assets_dir())?;

        let new_project_id = ProjectId::new();
        let restored_at = chrono::Utc::now();
        rewrite_identity(
            &new_paths.db_path(),
            new_project_id,
            &working_name,
            backup_manifest.project_id,
            backup_root,
            restored_at,
        )?;

        let new_manifest = Manifest {
            project_id: new_project_id,
            format_version: backup_manifest.format_version,
            schema_version: backup_manifest.schema_version,
            created_at: restored_at,
            working_name_cache: working_name.clone(),
            restored_from_project_id: Some(backup_manifest.project_id),
            restored_from_backup_id: Some(
                backup_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown-backup")
                    .to_string(),
            ),
        };
        new_manifest.write(&new_paths.manifest_path())?;
        validate_backup(&staging_root)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            fs::rename(&staging_root, &new_root)?;
            Ok(new_root)
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&staging_root);
            Err(e)
        }
    }
}

/// Rewrites `project_meta` in a not-yet-opened database copy with a new
/// Project ID and working name, in one transaction. This runs before any
/// `ProjectDbWorker` takes ownership of the connection, so there is never a
/// moment with two live owners of the same file.
fn rewrite_identity(
    db_path: &Path,
    new_project_id: ProjectId,
    working_name: &str,
    restored_from_project_id: ProjectId,
    backup_root: &Path,
    restored_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), BackupError> {
    let mut conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = FULL;")?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let backup_id = backup_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown-backup");
    let now = restored_at.to_rfc3339();
    let changed = tx.execute(
        "UPDATE project_meta SET
            project_id = ?1,
            working_name = ?2,
            restored_from_project_id = ?3,
            restored_from_backup_id = ?4,
            created_at = ?5,
            updated_at = ?5
         WHERE id = 1",
        rusqlite::params![
            new_project_id.to_string(),
            working_name,
            restored_from_project_id.to_string(),
            backup_id,
            now,
        ],
    )?;
    if changed != 1 {
        return Err(BackupError::CorruptSnapshot(db_path.display().to_string()));
    }

    tx.commit()?;
    Ok(())
}

fn canonical_or_lexical(path: &Path) -> Result<PathBuf, std::io::Error> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    resolve_prospective_path(&absolute, 0)
}

fn resolve_prospective_path(path: &Path, symlink_depth: usize) -> Result<PathBuf, std::io::Error> {
    if symlink_depth > 40 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "too many symlink levels",
        ));
    }

    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                resolved.push(component)
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            std::path::Component::Normal(part) => {
                resolved.push(part);
                match fs::symlink_metadata(&resolved) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        let target = fs::read_link(&resolved)?;
                        let target = if target.is_absolute() {
                            target
                        } else {
                            resolved
                                .parent()
                                .unwrap_or_else(|| Path::new(""))
                                .join(target)
                        };
                        resolved = resolve_prospective_path(&target, symlink_depth + 1)?;
                    }
                    Ok(_) => {
                        resolved = resolved.canonicalize()?;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => return Err(err),
                }
            }
        }
    }

    Ok(resolved)
}

fn ensure_outside_live_package(candidate: &Path, live: &PackagePaths) -> Result<(), BackupError> {
    let candidate = canonical_or_lexical(candidate)?;
    let root = canonical_or_lexical(&live.root)?;
    let final_path = candidate.join("backup-output.wcbackup");
    let staging_path = candidate.join(".backup-output.wcbackup.creating");
    if candidate == root
        || candidate.starts_with(&root)
        || final_path.starts_with(&root)
        || staging_path.starts_with(&root)
    {
        return Err(BackupError::UnsafePath(candidate.display().to_string()));
    }
    Ok(())
}

fn ensure_restore_destination_safe(backup: &Path, destination: &Path) -> Result<(), BackupError> {
    let backup = canonical_or_lexical(backup)?;
    let destination = canonical_or_lexical(destination)?;
    let final_path = destination.join("restored.wcproj");
    let staging_path = destination.join(".restore.creating");
    if destination == backup
        || destination.starts_with(&backup)
        || final_path.starts_with(&backup)
        || staging_path.starts_with(&backup)
    {
        return Err(BackupError::UnsafePath(destination.display().to_string()));
    }
    Ok(())
}

fn copy_dir_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    if !from.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package;
    use crate::persistence::worker::InitialProjectMeta;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn make_project(dir: &Path, name: &str) -> (ProjectId, PackagePaths, ProjectDbWorker) {
        let root = dir.join(format!("{name}.{}", layout::PACKAGE_EXTENSION));
        let paths = layout::create_skeleton(&root).unwrap();
        let project_id = ProjectId::new();
        let worker = ProjectDbWorker::spawn(
            paths.db_path(),
            project_id,
            Some(InitialProjectMeta {
                working_name: name.to_string(),
                format_version: package::FORMAT_VERSION,
            }),
        )
        .unwrap();
        let manifest = Manifest::new(project_id, package::FORMAT_VERSION, 1, name);
        manifest.write(&paths.manifest_path()).unwrap();
        (project_id, paths, worker)
    }

    #[test]
    fn backup_then_restore_creates_independent_project_with_new_id() {
        let dir = tempdir().unwrap();
        let (p1_id, p1_paths, worker) = make_project(dir.path(), "Tortuga");
        worker
            .rename_project(0, "Tortuga Renamed".to_string())
            .unwrap();

        let backup_root = dir.path().join("backups");
        let backup_path = create_backup(&worker, &p1_paths, &backup_root).unwrap();
        assert!(backup_path.join(package::layout::MANIFEST_FILE).is_file());

        let restore_dest = dir.path().join("restored");
        let p2_root = restore_as_copy(&backup_path, &restore_dest, Some("Tortuga Copy")).unwrap();

        let p2_manifest = Manifest::read(&p2_root.join(package::layout::MANIFEST_FILE)).unwrap();
        assert_ne!(p2_manifest.project_id, p1_id);
        assert_eq!(p2_manifest.restored_from_project_id, Some(p1_id));

        // P1 is unchanged: its own worker still reports its original ID and
        // the renamed working name.
        let p1_meta = worker.read_meta().unwrap();
        assert_eq!(p1_meta.project_id, p1_id);
        assert_eq!(p1_meta.working_name, "Tortuga Renamed");

        // P2's database identity matches its manifest.
        let conn = Connection::open(p2_root.join("data/project.sqlite")).unwrap();
        let db_id: String = conn
            .query_row("SELECT project_id FROM project_meta WHERE id=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(ProjectId::parse(&db_id).unwrap(), p2_manifest.project_id);

        worker.shutdown().unwrap();
    }

    #[test]
    fn corrupt_backup_is_rejected() {
        let dir = tempdir().unwrap();
        let backup_root = dir.path().join("bad.wcbackup");
        let paths = layout::create_skeleton(&backup_root).unwrap();
        let manifest = Manifest::new(ProjectId::new(), package::FORMAT_VERSION, 1, "Bad");
        manifest.write(&paths.manifest_path()).unwrap();
        fs::write(paths.db_path(), b"not a sqlite file").unwrap();

        let err = validate_backup(&backup_root).unwrap_err();
        assert!(matches!(err, BackupError::CorruptSnapshot(_)));

        let restore_dest = dir.path().join("restored");
        assert!(restore_as_copy(&backup_root, &restore_dest, None).is_err());
        assert!(!restore_dest.exists() || fs::read_dir(&restore_dest).unwrap().next().is_none());
    }

    #[test]
    fn backup_contains_committed_data_written_before_the_snapshot() {
        let dir = tempdir().unwrap();
        let (_p1_id, p1_paths, worker) = make_project(dir.path(), "Arak");
        worker
            .rename_project(0, "Arak Committed".to_string())
            .unwrap();

        let backup_root = dir.path().join("backups");
        let backup_path = create_backup(&worker, &p1_paths, &backup_root).unwrap();

        let conn = Connection::open(backup_path.join("data/project.sqlite")).unwrap();
        let name: String = conn
            .query_row(
                "SELECT working_name FROM project_meta WHERE id=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Arak Committed");

        worker.shutdown().unwrap();
    }

    #[test]
    fn prospective_normalization_collapses_nonexistent_parent_traversal() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("safe/../sibling/new-output");
        assert_eq!(
            canonical_or_lexical(&path).unwrap(),
            dir.path()
                .canonicalize()
                .unwrap()
                .join("sibling/new-output")
        );
    }

    #[cfg(unix)]
    #[test]
    fn prospective_normalization_resolves_symlink_parents_component_by_component() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("real/nested")).unwrap();
        symlink("real/nested", dir.path().join("alias")).unwrap();

        let path = dir.path().join("alias/../future/output");
        assert_eq!(
            canonical_or_lexical(&path).unwrap(),
            dir.path()
                .canonicalize()
                .unwrap()
                .join("real/future/output")
        );
    }

    #[test]
    fn containment_allows_a_safe_sibling_and_rejects_a_nested_destination() {
        let dir = tempdir().unwrap();
        let (_, paths, worker) = make_project(dir.path(), "Tortuga");
        assert!(ensure_outside_live_package(&dir.path().join("backups"), &paths).is_ok());
        assert!(
            ensure_outside_live_package(&paths.assets_dir().join("../nested"), &paths).is_err()
        );
        worker.shutdown().unwrap();
    }
}
