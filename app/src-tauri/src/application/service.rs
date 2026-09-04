//! Named application commands: Create/Open/Rename/Close Project, and the
//! manual backup / Restore-as-Copy foundation.
//!
//! Each function here owns one use case end-to-end, including cleaning up
//! after itself on failure so a Project is never left half-created.

use std::path::{Path, PathBuf};

use crate::domain::{
    authored_name, require_definition_name, Category, CategoryId, Entry, EntryId, ProjectId,
    TypeDef, TypeId, WorkingName,
};
use crate::package::{self, layout, manifest::Manifest, PackagePaths};
use crate::persistence::lock::{self, LockGuard};
use crate::persistence::worker::InitialProjectMeta;
use crate::persistence::{PersistenceError, ProjectDbWorker};

use super::error::AppError;
use super::state::{AppState, OpenProject, ProjectSummary};

pub struct ProjectService;

impl ProjectService {
    /// Creates a brand-new Project package under `base_dir`, opens it, and
    /// registers it in `state`. On any failure the partially created
    /// package is removed so a failed creation never leaves debris behind.
    pub fn create_project(
        state: &AppState,
        base_dir: &Path,
        working_name_raw: &str,
    ) -> Result<ProjectSummary, AppError> {
        let working_name = WorkingName::new(working_name_raw)?;
        let project_id = ProjectId::new();
        let root = layout::available_package_path(base_dir, working_name.as_str());

        let paths = package::layout::create_skeleton(&root)?;

        let result = (|| -> Result<ProjectSummary, AppError> {
            let worker = ProjectDbWorker::spawn(
                paths.db_path(),
                project_id,
                Some(InitialProjectMeta {
                    working_name: working_name.as_str().to_string(),
                    format_version: package::FORMAT_VERSION,
                }),
            )?;

            let manifest = Manifest::new(
                project_id,
                package::FORMAT_VERSION,
                crate::persistence::migrations::CURRENT_SCHEMA_VERSION,
                working_name.as_str(),
            );
            manifest.write(&paths.manifest_path())?;

            let lock_guard = lock::acquire(&paths.lock_path(), project_id, false)?;

            let summary = summary_from_worker(&worker, &paths)?;

            register_open_project(state, project_id, worker, paths.clone(), lock_guard);

            Ok(summary)
        })();

        match result {
            Ok(summary) => Ok(summary),
            Err(e) => {
                // Creation failed after the skeleton was written: clean up
                // so the caller can retry without leftover debris.
                let _ = std::fs::remove_dir_all(&root);
                Err(e)
            }
        }
    }

    /// Opens an existing Project package, validating structure, format
    /// version, and manifest/database identity match, and acquiring the
    /// exclusive Project lock.
    pub fn open_project(
        state: &AppState,
        package_root: &Path,
        force_stale_lock_recovery: bool,
    ) -> Result<ProjectSummary, AppError> {
        let paths = package::layout::validate_structure(package_root)?;
        let mut manifest = Manifest::read(&paths.manifest_path())?;
        ensure_manifest_is_writable(&manifest)?;
        let preflight = ProjectDbWorker::preflight_existing(paths.db_path(), manifest.project_id)?;
        if manifest.schema_version > preflight.schema_version {
            return Err(AppError::Persistence(PersistenceError::Other(
                "manifest schema version is ahead of the database; refusing unsafe recovery"
                    .to_string(),
            )));
        }

        let lock_guard = lock::acquire(
            &paths.lock_path(),
            manifest.project_id,
            force_stale_lock_recovery,
        )?;

        let result = (|| -> Result<(ProjectDbWorker, ProjectSummary), AppError> {
            if preflight.schema_version < crate::persistence::migrations::CURRENT_SCHEMA_VERSION {
                crate::backup_recovery::create_pre_migration_snapshot(&paths, &manifest)?;
            }
            let worker = ProjectDbWorker::spawn(paths.db_path(), manifest.project_id, None)?;
            let summary = summary_from_worker(&worker, &paths)?;
            if summary.format_version != manifest.format_version {
                return Err(AppError::Persistence(
                    crate::persistence::PersistenceError::Other(
                        "manifest and database format versions disagree".to_string(),
                    ),
                ));
            }
            if manifest.schema_version != summary.schema_version {
                manifest.schema_version = summary.schema_version;
                manifest.working_name_cache = summary.working_name.clone();
                if let Err(error) = manifest.write(&paths.manifest_path()) {
                    let _ = worker.shutdown();
                    return Err(error.into());
                }
            }
            Ok((worker, summary))
        })();

        match result {
            Ok((worker, summary)) => {
                register_open_project(state, manifest.project_id, worker, paths, lock_guard);
                Ok(summary)
            }
            Err(e) => {
                // Opening failed after the lock was acquired: release it so
                // the Project is not left artificially locked.
                lock_guard.release();
                Err(e)
            }
        }
    }

    /// Renames the visible working name only. The Project ID, database
    /// identity, and package-internal references are untouched, and the
    /// `.wcproj` directory itself is never renamed as a side effect.
    pub fn rename_project(
        state: &AppState,
        project_id: ProjectId,
        new_name_raw: &str,
        expected_revision: i64,
    ) -> Result<ProjectSummary, AppError> {
        let new_name = WorkingName::new(new_name_raw)?;
        let open = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .get(&project_id)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(project_id))?;
        let outcome = {
            let worker = open.worker.lock().expect("worker mutex poisoned");
            worker
                .as_ref()
                .ok_or(AppError::ProjectNotOpen(project_id))?
                .rename_project(expected_revision, new_name.as_str().to_string())?
        };

        // Best-effort cache refresh: the database row is already the
        // committed source of truth, so a failure to refresh the manifest
        // cache is not itself a Saved failure.
        if let Ok(mut manifest) = Manifest::read(&open.paths.manifest_path()) {
            manifest.working_name_cache = new_name.as_str().to_string();
            let _ = manifest.write(&open.paths.manifest_path());
        }

        Ok(ProjectSummary {
            project_id,
            working_name: new_name.into_string(),
            revision: outcome.committed_revision,
            package_path: open.paths.root.display().to_string(),
            format_version: package::FORMAT_VERSION,
            schema_version: crate::persistence::migrations::CURRENT_SCHEMA_VERSION,
            created_at: read_created_at(&open, project_id)?,
            updated_at: outcome.updated_at,
        })
    }

    /// Closes a Project: shuts down its worker (draining any queued
    /// commands first) and releases its lock. Callers are responsible for
    /// ensuring no pending/dirty UI work is discarded before calling this
    /// (see the frontend Saved-state contract).
    pub fn close_project(state: &AppState, project_id: ProjectId) -> Result<(), AppError> {
        let open = {
            let mut registry = state.open_projects.lock().expect("registry mutex poisoned");
            registry
                .remove(&project_id)
                .ok_or(AppError::ProjectNotOpen(project_id))?
        };
        let worker = open.worker.lock().expect("worker mutex poisoned").take();
        if let Some(worker) = worker {
            worker.shutdown()?;
        }
        if let Some(lock) = open.lock.lock().expect("lock mutex poisoned").take() {
            lock.release();
        }
        Ok(())
    }

    /// Reads the current summary of an open Project without mutating it.
    pub fn get_summary(
        state: &AppState,
        project_id: ProjectId,
    ) -> Result<ProjectSummary, AppError> {
        let open = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .get(&project_id)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(project_id))?;
        let worker = open.worker.lock().expect("worker mutex poisoned");
        summary_from_worker(
            worker
                .as_ref()
                .ok_or(AppError::ProjectNotOpen(project_id))?,
            &open.paths,
        )
    }

    pub fn list_categories(
        state: &AppState,
        project_id: ProjectId,
    ) -> Result<Vec<Category>, AppError> {
        Self::with_worker(state, project_id, |worker| worker.list_categories())
    }

    pub fn create_category(
        state: &AppState,
        project_id: ProjectId,
        name: &str,
    ) -> Result<Category, AppError> {
        let name = require_definition_name(name)?;
        Self::with_worker(state, project_id, |worker| {
            worker.create_category(CategoryId::new(), name)
        })
    }

    pub fn list_types(
        state: &AppState,
        project_id: ProjectId,
        category_id: CategoryId,
    ) -> Result<Vec<TypeDef>, AppError> {
        Self::with_worker(state, project_id, |worker| worker.list_types(category_id))
    }

    pub fn create_type(
        state: &AppState,
        project_id: ProjectId,
        category_id: CategoryId,
        parent_type_id: Option<TypeId>,
        name: &str,
    ) -> Result<TypeDef, AppError> {
        let name = require_definition_name(name)?;
        Self::with_worker(state, project_id, |worker| {
            worker.create_type(TypeId::new(), category_id, parent_type_id, name)
        })
    }

    pub fn list_entries(state: &AppState, project_id: ProjectId) -> Result<Vec<Entry>, AppError> {
        Self::with_worker(state, project_id, |worker| worker.list_entries())
    }

    pub fn create_entry(
        state: &AppState,
        project_id: ProjectId,
        category_id: Option<CategoryId>,
        type_id: Option<TypeId>,
        name: Option<String>,
    ) -> Result<Entry, AppError> {
        let name = authored_name(name);
        Self::with_worker(state, project_id, |worker| {
            worker.create_entry(EntryId::new(), category_id, type_id, name)
        })
    }

    pub fn get_entry(
        state: &AppState,
        project_id: ProjectId,
        entry_id: EntryId,
    ) -> Result<Entry, AppError> {
        Self::with_worker(state, project_id, |worker| worker.get_entry(entry_id))
    }

    pub fn update_entry_name(
        state: &AppState,
        project_id: ProjectId,
        entry_id: EntryId,
        expected_revision: i64,
        name: Option<String>,
    ) -> Result<Entry, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.update_entry_name(entry_id, expected_revision, name)
        })
    }

    pub fn change_entry_structure(
        state: &AppState,
        project_id: ProjectId,
        entry_id: EntryId,
        expected_revision: i64,
        category_id: CategoryId,
        type_id: Option<TypeId>,
    ) -> Result<Entry, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.change_entry_structure(entry_id, expected_revision, category_id, type_id)
        })
    }

    /// Creates a manual, consistent backup of an open Project outside its
    /// live package.
    pub fn create_backup(
        state: &AppState,
        project_id: ProjectId,
        backup_root: &Path,
    ) -> Result<PathBuf, AppError> {
        let open = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .get(&project_id)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(project_id))?;
        let worker = open.worker.lock().expect("worker mutex poisoned");
        crate::backup_recovery::create_backup(
            worker
                .as_ref()
                .ok_or(AppError::ProjectNotOpen(project_id))?,
            &open.paths,
            backup_root,
        )
        .map_err(AppError::from)
    }

    fn with_worker<T>(
        state: &AppState,
        project_id: ProjectId,
        action: impl FnOnce(&ProjectDbWorker) -> Result<T, PersistenceError>,
    ) -> Result<T, AppError> {
        let open = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .get(&project_id)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(project_id))?;
        let worker = open.worker.lock().expect("worker mutex poisoned");
        action(
            worker
                .as_ref()
                .ok_or(AppError::ProjectNotOpen(project_id))?,
        )
        .map_err(AppError::from)
    }

    /// Restores a validated backup as an independent new Project (new
    /// Project ID), registering it as open on success. The source backup
    /// and its originating live Project are never modified.
    pub fn restore_backup_as_copy(
        state: &AppState,
        backup_path: &Path,
        destination_dir: &Path,
        new_working_name: Option<&str>,
    ) -> Result<ProjectSummary, AppError> {
        let backup_manifest = crate::backup_recovery::validate_backup(backup_path)?;
        ensure_manifest_is_writable(&backup_manifest)?;
        let new_root = crate::backup_recovery::restore_as_copy(
            backup_path,
            destination_dir,
            new_working_name,
        )?;
        Self::open_project(state, &new_root, false)
    }
}

fn read_created_at(
    open: &OpenProject,
    project_id: ProjectId,
) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
    Ok(open
        .worker
        .lock()
        .expect("worker mutex poisoned")
        .as_ref()
        .ok_or(AppError::ProjectNotOpen(project_id))?
        .read_meta()?
        .created_at)
}

fn summary_from_worker(
    worker: &ProjectDbWorker,
    paths: &PackagePaths,
) -> Result<ProjectSummary, AppError> {
    let meta = worker.read_meta()?;
    Ok(ProjectSummary {
        project_id: meta.project_id,
        working_name: meta.working_name,
        revision: meta.last_committed_revision,
        package_path: paths.root.display().to_string(),
        format_version: meta.format_version,
        schema_version: meta.schema_version,
        created_at: meta.created_at,
        updated_at: meta.updated_at,
    })
}

fn ensure_manifest_is_writable(manifest: &Manifest) -> Result<(), AppError> {
    if manifest.format_version > package::FORMAT_VERSION {
        return Err(AppError::Package(
            package::PackageError::UnsupportedFormatVersion {
                found: manifest.format_version,
                supported: package::FORMAT_VERSION,
            },
        ));
    }

    if manifest.schema_version > crate::persistence::migrations::CURRENT_SCHEMA_VERSION {
        return Err(AppError::Persistence(
            PersistenceError::UnsupportedSchemaVersion {
                found: manifest.schema_version,
                supported: crate::persistence::migrations::CURRENT_SCHEMA_VERSION,
            },
        ));
    }

    Ok(())
}

fn register_open_project(
    state: &AppState,
    project_id: ProjectId,
    worker: ProjectDbWorker,
    paths: PackagePaths,
    lock: LockGuard,
) {
    let mut registry = state.open_projects.lock().expect("registry mutex poisoned");
    registry.insert(
        project_id,
        std::sync::Arc::new(OpenProject {
            worker: std::sync::Mutex::new(Some(worker)),
            paths,
            lock: std::sync::Mutex::new(Some(lock)),
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_project_produces_matching_manifest_and_database_ids() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let summary = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();

        let manifest =
            Manifest::read(&Path::new(&summary.package_path).join(package::layout::MANIFEST_FILE))
                .unwrap();
        assert_eq!(manifest.project_id, summary.project_id);
        assert_eq!(summary.working_name, "Tortuga");
        assert_eq!(summary.revision, 0);

        ProjectService::close_project(&state, summary.project_id).unwrap();
    }

    #[test]
    fn rename_keeps_id_stable_and_survives_close_reopen() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();

        let renamed = ProjectService::rename_project(
            &state,
            created.project_id,
            "Tortuga Prime",
            created.revision,
        )
        .unwrap();
        assert_eq!(renamed.project_id, created.project_id);
        assert_eq!(renamed.working_name, "Tortuga Prime");

        let package_path = PathBuf::from(&created.package_path);
        ProjectService::close_project(&state, created.project_id).unwrap();

        let reopened = ProjectService::open_project(&state, &package_path, false).unwrap();
        assert_eq!(reopened.project_id, created.project_id);
        assert_eq!(reopened.working_name, "Tortuga Prime");
        ProjectService::close_project(&state, reopened.project_id).unwrap();
    }

    #[test]
    fn stale_rename_revision_is_rejected() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
        ProjectService::rename_project(&state, created.project_id, "First", created.revision)
            .unwrap();

        let err =
            ProjectService::rename_project(&state, created.project_id, "Stale", created.revision)
                .unwrap_err();
        assert_eq!(err.kind(), "revision_conflict");
        ProjectService::close_project(&state, created.project_id).unwrap();
    }

    #[test]
    fn a_second_open_is_refused_while_the_first_is_open() {
        let state_a = AppState::default();
        let state_b = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state_a, dir.path(), "Tortuga").unwrap();
        let package_path = PathBuf::from(&created.package_path);

        let err = ProjectService::open_project(&state_b, &package_path, false).unwrap_err();
        assert_eq!(err.kind(), "lock_held");

        ProjectService::close_project(&state_a, created.project_id).unwrap();
        // Normal close releases the lock, so a subsequent open succeeds.
        let reopened = ProjectService::open_project(&state_b, &package_path, false).unwrap();
        ProjectService::close_project(&state_b, reopened.project_id).unwrap();
    }

    #[test]
    fn opening_a_package_with_a_tampered_manifest_id_is_rejected() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
        let package_path = PathBuf::from(&created.package_path);
        ProjectService::close_project(&state, created.project_id).unwrap();

        let paths = PackagePaths::new(&package_path);
        let mut manifest = Manifest::read(&paths.manifest_path()).unwrap();
        manifest.project_id = ProjectId::new();
        manifest.write(&paths.manifest_path()).unwrap();

        let err = ProjectService::open_project(&state, &package_path, false).unwrap_err();
        assert_eq!(err.kind(), "identity_mismatch");
    }

    #[test]
    fn create_backup_then_restore_as_copy_leaves_original_untouched() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
        ProjectService::rename_project(&state, created.project_id, "Renamed", created.revision)
            .unwrap();

        let backup_root = dir.path().join("backups");
        let backup_path =
            ProjectService::create_backup(&state, created.project_id, &backup_root).unwrap();

        let restore_dir = dir.path().join("restored");
        let restored = ProjectService::restore_backup_as_copy(
            &state,
            &backup_path,
            &restore_dir,
            Some("Tortuga Copy"),
        )
        .unwrap();
        assert_ne!(restored.project_id, created.project_id);
        assert_eq!(restored.working_name, "Tortuga Copy");

        let original = ProjectService::get_summary(&state, created.project_id).unwrap();
        assert_eq!(original.working_name, "Renamed");

        ProjectService::close_project(&state, created.project_id).unwrap();
        ProjectService::close_project(&state, restored.project_id).unwrap();
    }

    #[test]
    fn schema_v1_package_migrates_and_reopens_with_identity_and_name_unchanged() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
        let package_path = PathBuf::from(&created.package_path);
        ProjectService::close_project(&state, created.project_id).unwrap();

        let paths = PackagePaths::new(&package_path);
        let mut manifest = Manifest::read(&paths.manifest_path()).unwrap();
        manifest.schema_version = 1;
        manifest.write(&paths.manifest_path()).unwrap();

        let conn = rusqlite::Connection::open(paths.db_path()).unwrap();
        conn.execute_batch(
            "DROP TRIGGER entry_type_category_update;
             DROP TRIGGER entry_type_category_insert;
             DROP TABLE entry;
             DROP TABLE record_identity;
             DROP TRIGGER type_parent_cycle_update;
             DROP TRIGGER type_parent_valid_insert;
             DROP TABLE type_def;
             DROP TABLE category;",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute(
            "UPDATE project_meta SET schema_version = 1 WHERE id = 1",
            [],
        )
        .unwrap();
        drop(conn);

        let reopened = ProjectService::open_project(&state, &package_path, false).unwrap();
        assert_eq!(reopened.project_id, created.project_id);
        assert_eq!(reopened.working_name, created.working_name);
        assert_eq!(
            reopened.schema_version,
            crate::persistence::migrations::CURRENT_SCHEMA_VERSION
        );
        let manifest_after = Manifest::read(&paths.manifest_path()).unwrap();
        assert_eq!(manifest_after.schema_version, reopened.schema_version);
        let recovery = package_path
            .parent()
            .unwrap()
            .join(".worldcrafter-migration-recovery")
            .join(created.project_id.to_string())
            .join("schema-v1.sqlite");
        assert!(recovery.is_file());
        ProjectService::close_project(&state, reopened.project_id).unwrap();
    }

    #[test]
    fn committed_database_with_old_manifest_is_recovered_on_next_open() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
        let package_path = PathBuf::from(&created.package_path);
        ProjectService::close_project(&state, created.project_id).unwrap();
        let paths = PackagePaths::new(&package_path);
        let mut manifest = Manifest::read(&paths.manifest_path()).unwrap();
        manifest.schema_version = 1;
        manifest.write(&paths.manifest_path()).unwrap();

        let reopened = ProjectService::open_project(&state, &package_path, false).unwrap();
        assert_eq!(reopened.project_id, created.project_id);
        assert_eq!(
            Manifest::read(&paths.manifest_path())
                .unwrap()
                .schema_version,
            crate::persistence::migrations::CURRENT_SCHEMA_VERSION
        );
        ProjectService::close_project(&state, reopened.project_id).unwrap();
    }

    #[test]
    fn failed_migration_keeps_v1_recoverable_and_never_registers_open() {
        let state = AppState::default();
        let dir = tempdir().unwrap();
        let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
        let package_path = PathBuf::from(&created.package_path);
        ProjectService::close_project(&state, created.project_id).unwrap();
        let paths = PackagePaths::new(&package_path);
        let mut manifest = Manifest::read(&paths.manifest_path()).unwrap();
        manifest.schema_version = 1;
        manifest.write(&paths.manifest_path()).unwrap();
        let conn = rusqlite::Connection::open(paths.db_path()).unwrap();
        conn.execute_batch(
            "DROP TRIGGER entry_type_category_update;
             DROP TRIGGER entry_type_category_insert;
             DROP TABLE entry;
             DROP TABLE record_identity;
             DROP TRIGGER type_parent_cycle_update;
             DROP TRIGGER type_parent_valid_insert;
             DROP TABLE type_def;
             DROP TABLE category;
             PRAGMA user_version = 1;
             UPDATE project_meta SET schema_version = 1;
             CREATE TRIGGER force_migration_failure
             BEFORE UPDATE OF schema_version ON project_meta
             BEGIN SELECT RAISE(ABORT, 'simulated migration interruption'); END;",
        )
        .unwrap();
        drop(conn);

        assert!(ProjectService::open_project(&state, &package_path, false).is_err());
        assert!(!state
            .open_projects
            .lock()
            .unwrap()
            .contains_key(&created.project_id));
        assert_eq!(
            Manifest::read(&paths.manifest_path())
                .unwrap()
                .schema_version,
            1
        );
        let conn = rusqlite::Connection::open(paths.db_path()).unwrap();
        assert_eq!(
            crate::persistence::migrations::user_version(&conn).unwrap(),
            1
        );
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        conn.execute("DROP TRIGGER force_migration_failure", [])
            .unwrap();
        drop(conn);

        let recovered = ProjectService::open_project(&state, &package_path, false).unwrap();
        assert_eq!(recovered.project_id, created.project_id);
        ProjectService::close_project(&state, recovered.project_id).unwrap();
    }
}
