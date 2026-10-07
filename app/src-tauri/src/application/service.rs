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
    pub fn preview_category_delete(
        state: &AppState,
        project: ProjectId,
        id: CategoryId,
    ) -> Result<crate::domain::lifecycle::CategoryDeletePreview, AppError> {
        Self::with_worker(state, project, |w| w.preview_category_delete(id))
    }
    pub fn apply_structure(
        state: &AppState,
        project: ProjectId,
        expected: i64,
        command: crate::domain::lifecycle::StructureCommand,
        backup_root: Option<&Path>,
    ) -> Result<crate::domain::lifecycle::StructureOutcome, AppError> {
        use crate::domain::lifecycle::StructureCommand;
        let open = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .get(&project)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(project))?;
        let guard = open.worker.lock().expect("worker mutex poisoned");
        let worker = guard.as_ref().ok_or(AppError::ProjectNotOpen(project))?;
        let current = worker.read_meta()?.last_committed_revision;
        if current != expected {
            return Err(PersistenceError::StaleRevision { expected, current }.into());
        }
        let backup = if let StructureCommand::DeleteCategory {
            id,
            remove_types,
            destination_id,
        } = &command
        {
            let preview = worker.preview_category_delete(*id)?;
            if !preview.type_names.is_empty() && !remove_types {
                return Err(PersistenceError::Other(
                    "Confirm removal of the reviewed Types first".into(),
                )
                .into());
            }
            if id == destination_id
                || !worker
                    .list_categories()?
                    .iter()
                    .any(|c| c.id == *destination_id)
            {
                return Err(PersistenceError::Other(
                    "Choose another Category for these Entries".into(),
                )
                .into());
            }
            let root = backup_root.ok_or_else(|| {
                PersistenceError::Other("A recovery backup location is required".into())
            })?;
            Some(
                crate::backup_recovery::create_backup(worker, &open.paths, root)?
                    .display()
                    .to_string(),
            )
        } else {
            None
        };
        let mut result = worker.apply_structure(expected, command)?;
        result.backup_path = backup;
        Ok(result)
    }

    pub fn read_timeline(
        state: &AppState,
        project_id: ProjectId,
    ) -> Result<crate::domain::timeline::TimelineSnapshot, AppError> {
        Self::with_worker(state, project_id, |w| w.read_timeline())
    }
    pub fn apply_timeline(
        state: &AppState,
        project_id: ProjectId,
        expected: i64,
        command: crate::domain::timeline::TimelineCommand,
    ) -> Result<crate::domain::timeline::TimelineSnapshot, AppError> {
        Self::with_worker(state, project_id, |w| w.apply_timeline(expected, command))
    }
    pub fn search_project(
        state: &AppState,
        project_id: ProjectId,
        request: crate::domain::search::SearchRequest,
    ) -> Result<crate::domain::search::SearchResults, AppError> {
        Self::with_worker(state, project_id, |w| w.search_project(request))
    }
    pub fn read_aliases(
        state: &AppState,
        project_id: ProjectId,
        entry_id: EntryId,
    ) -> Result<crate::domain::search::EntryAliases, AppError> {
        Self::with_worker(state, project_id, |w| w.read_aliases(entry_id))
    }
    pub fn apply_alias(
        state: &AppState,
        project_id: ProjectId,
        entry_id: EntryId,
        expected: i64,
        command: crate::domain::search::AliasCommand,
    ) -> Result<crate::domain::search::EntryAliases, AppError> {
        Self::with_worker(state, project_id, |w| {
            w.apply_alias(entry_id, expected, command)
        })
    }
    pub fn read_story(
        state: &AppState,
        project_id: ProjectId,
    ) -> Result<crate::domain::story::StoryIndex, AppError> {
        Self::with_worker(state, project_id, |worker| worker.read_story())
    }
    pub fn read_chapter(
        state: &AppState,
        project_id: ProjectId,
        chapter_id: crate::domain::structure::ChapterId,
    ) -> Result<crate::domain::story::ChapterSnapshot, AppError> {
        Self::with_worker(state, project_id, |worker| worker.read_chapter(chapter_id))
    }
    pub fn story_usage(
        state: &AppState,
        project_id: ProjectId,
        entry_id: EntryId,
    ) -> Result<Vec<crate::domain::story::StoryUsage>, AppError> {
        Self::with_worker(state, project_id, |worker| worker.story_usage(entry_id))
    }
    pub fn apply_story(
        state: &AppState,
        project_id: ProjectId,
        expected: i64,
        command: crate::domain::story::StoryCommand,
    ) -> Result<crate::domain::story::ChapterSnapshot, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.apply_story(expected, command)
        })
    }
    pub fn read_spatial(
        state: &AppState,
        project_id: ProjectId,
    ) -> Result<crate::domain::spatial::SpatialSnapshot, AppError> {
        Self::with_worker(state, project_id, |worker| worker.read_spatial())
    }
    pub fn apply_spatial(
        state: &AppState,
        project_id: ProjectId,
        expected: i64,
        command: crate::domain::spatial::SpatialCommand,
    ) -> Result<crate::domain::spatial::SpatialSnapshot, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.apply_spatial(expected, command)
        })
    }

    pub fn delete_entry_field(
        state: &AppState,
        project: ProjectId,
        entry: EntryId,
        field: crate::domain::structure::FieldId,
        expected: i64,
        backup_root: &Path,
    ) -> Result<crate::domain::fields::EntryFieldDeleteOutcome, AppError> {
        let open = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .get(&project)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(project))?;
        let guard = open.worker.lock().expect("worker mutex poisoned");
        let worker = guard.as_ref().ok_or(AppError::ProjectNotOpen(project))?;
        let snapshot = worker.read_fields(entry)?;
        if snapshot.global_revision != expected {
            return Err(PersistenceError::StaleRevision {
                expected,
                current: snapshot.global_revision,
            }
            .into());
        }
        if !snapshot.fields.iter().any(|f| f.definition.id == field) {
            return Err(PersistenceError::Other("Field is no longer on this Entry".into()).into());
        }
        if snapshot
            .fields
            .iter()
            .any(|f| f.definition.id == field && f.definition.projection.is_some())
        {
            return Err(PersistenceError::Other("Remove the Field display or explicitly end its connections; scalar deletion cannot delete a relationship".into()).into());
        }
        let backup = crate::backup_recovery::create_backup(worker, &open.paths, backup_root)?;
        let snapshot = worker.delete_entry_field(entry, field, expected)?;
        Ok(crate::domain::fields::EntryFieldDeleteOutcome {
            snapshot,
            backup_path: backup.display().to_string(),
        })
    }

    pub fn preview_field_merge(
        state: &AppState,
        project: ProjectId,
        source: crate::domain::structure::FieldId,
        target: crate::domain::structure::FieldId,
    ) -> Result<crate::domain::fields::FieldMergePreview, AppError> {
        Self::with_worker(state, project, |worker| {
            worker.preview_field_merge(source, target)
        })
    }
    pub fn merge_fields(
        state: &AppState,
        project: ProjectId,
        source: crate::domain::structure::FieldId,
        target: crate::domain::structure::FieldId,
        expected: i64,
        backup_root: &Path,
    ) -> Result<crate::domain::fields::FieldMergeOutcome, AppError> {
        let open = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .get(&project)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(project))?;
        let guard = open.worker.lock().expect("worker mutex poisoned");
        let worker = guard.as_ref().ok_or(AppError::ProjectNotOpen(project))?;
        let preview = worker.preview_field_merge(source, target)?;
        if preview.global_revision != expected {
            return Err(PersistenceError::StaleRevision {
                expected,
                current: preview.global_revision,
            }
            .into());
        }
        if !preview.blockers.is_empty() {
            return Err(PersistenceError::Other(preview.blockers.join(" ")).into());
        }
        // Nothing can write or close this Project between the validated backup
        // and transaction. A backup failure aborts before any authored change.
        let backup = crate::backup_recovery::create_backup(worker, &open.paths, backup_root)?;
        let revision = worker.merge_fields(source, target, expected)?;
        Ok(crate::domain::fields::FieldMergeOutcome {
            global_revision: revision,
            backup_path: backup.display().to_string(),
        })
    }

    pub fn read_field_catalog(
        state: &AppState,
        project_id: ProjectId,
    ) -> Result<crate::domain::fields::FieldCatalog, AppError> {
        Self::with_worker(state, project_id, |worker| worker.read_field_catalog())
    }
    pub fn apply_template_fields(
        state: &AppState,
        project_id: ProjectId,
        expected: i64,
        command: crate::domain::fields::FieldCommand,
    ) -> Result<crate::domain::fields::FieldCatalog, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.apply_template_fields(expected, command)
        })
    }

    pub fn read_project_relationships(
        state: &AppState,
        project_id: ProjectId,
    ) -> Result<crate::domain::relationships::RelationshipSnapshot, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.read_project_relationships()
        })
    }

    pub fn read_relationships(
        state: &AppState,
        project_id: ProjectId,
        entry: EntryId,
    ) -> Result<crate::domain::relationships::EntryRelationships, AppError> {
        Self::with_worker(state, project_id, |worker| worker.read_relationships(entry))
    }
    pub fn apply_relationships(
        state: &AppState,
        project_id: ProjectId,
        entry: EntryId,
        expected: i64,
        command: crate::domain::relationships::RelationshipCommand,
    ) -> Result<crate::domain::relationships::EntryRelationships, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.apply_relationships(entry, expected, command)
        })
    }
    pub fn read_fields(
        state: &AppState,
        project_id: ProjectId,
        entry: EntryId,
    ) -> Result<crate::domain::fields::EntryFields, AppError> {
        Self::with_worker(state, project_id, |worker| worker.read_fields(entry))
    }
    pub fn apply_fields(
        state: &AppState,
        project_id: ProjectId,
        entry: EntryId,
        expected: i64,
        command: crate::domain::fields::FieldCommand,
    ) -> Result<crate::domain::fields::EntryFields, AppError> {
        Self::with_worker(state, project_id, |worker| {
            worker.apply_fields(entry, expected, command)
        })
    }
    /// Creates a brand-new Project package under `base_dir`, opens it, and
    /// registers it in `state`. The candidate package path is always
    /// `<working name>.wcproj`; a collision is never silently side-stepped
    /// (see [`PackageError::AlreadyExists`]) so the caller can visibly
    /// report it and let the user choose another name or location. On any
    /// other failure the partially created package is removed so a failed
    /// creation never leaves debris behind.
    pub fn create_project(
        state: &AppState,
        base_dir: &Path,
        working_name_raw: &str,
    ) -> Result<ProjectSummary, AppError> {
        let _lifecycle = state.lifecycle.lock().expect("lifecycle mutex poisoned");
        let working_name = WorkingName::new(working_name_raw)?;
        let project_id = ProjectId::new();
        let root = layout::single_candidate_package_path(base_dir, working_name.as_str());

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

            register_open_project(
                state,
                project_id,
                worker,
                paths.clone(),
                lock_guard,
                summary.revision,
            );

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
        Self::open_expected_project(state, package_root, force_stale_lock_recovery, None)
    }

    /// Recent-project repair must never open or migrate a different Project.
    pub fn open_expected_project(
        state: &AppState,
        package_root: &Path,
        force_stale_lock_recovery: bool,
        expected: Option<ProjectId>,
    ) -> Result<ProjectSummary, AppError> {
        let _lifecycle = state.lifecycle.lock().expect("lifecycle mutex poisoned");
        let canonical_root = std::fs::canonicalize(package_root)?;
        let live = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .iter()
            .map(|(id, open)| (*id, open.clone()))
            .collect::<Vec<_>>();
        for (id, open) in &live {
            if std::fs::canonicalize(&open.paths.root).is_ok_and(|path| path == canonical_root) {
                if expected.is_some_and(|expected| expected != *id) {
                    return Err(AppError::RecentProjectMismatch);
                }
                // Only a worker owned by this AppState can be resumed. Never infer
                // ownership from lock.json's PID or remove/reacquire its live lock.
                ProjectDbWorker::preflight_existing(open.paths.db_path(), *id)?;
                return Self::get_summary(state, *id);
            }
        }
        if let Some(expected) = expected {
            // Structure validation can recover an interrupted manifest publication.
            // Verify a recent shortcut's identity read-only before touching that file.
            ProjectDbWorker::preflight_existing(
                PackagePaths::new(package_root).db_path(),
                expected,
            )
            .map_err(|error| match error {
                PersistenceError::ProjectIdMismatch { .. } => AppError::RecentProjectMismatch,
                other => other.into(),
            })?;
        }
        let paths = package::layout::validate_structure(package_root)?;
        let mut manifest = Manifest::read(&paths.manifest_path())?;
        if let Some(expected) = expected {
            if expected != manifest.project_id {
                return Err(AppError::RecentProjectMismatch);
            }
        }
        if live.iter().any(|(id, _)| *id == manifest.project_id) {
            return Err(AppError::DuplicateOpenProject);
        }
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
                crate::backup_recovery::create_pre_migration_snapshot(
                    &paths,
                    manifest.project_id,
                    preflight.schema_version,
                )?;
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
                register_open_project(
                    state,
                    manifest.project_id,
                    worker,
                    paths,
                    lock_guard,
                    summary.revision,
                );
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
        let _lifecycle = state.lifecycle.lock().expect("lifecycle mutex poisoned");
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

    /// Renderer reloads do not close Rust workers. Reconnect to their committed
    /// state rather than making the Home screen try to acquire their locks again.
    pub fn list_open_projects(state: &AppState) -> Result<Vec<ProjectSummary>, AppError> {
        let _lifecycle = state.lifecycle.lock().expect("lifecycle mutex poisoned");
        let ids = state
            .open_projects
            .lock()
            .expect("registry mutex poisoned")
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut summaries = ids
            .into_iter()
            .map(|id| Self::get_summary(state, id))
            .collect::<Result<Vec<_>, _>>()?;
        summaries.sort_by_key(|summary| summary.project_id.to_string());
        Ok(summaries)
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
    opened_revision: i64,
) {
    let mut registry = state.open_projects.lock().expect("registry mutex poisoned");
    registry.insert(
        project_id,
        std::sync::Arc::new(OpenProject {
            worker: std::sync::Mutex::new(Some(worker)),
            paths,
            lock: std::sync::Mutex::new(Some(lock)),
            opened_revision,
            opened_at: std::time::Instant::now(),
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
        let category =
            ProjectService::create_category(&state, created.project_id, "Characters").unwrap();
        let type_def =
            ProjectService::create_type(&state, created.project_id, category.id, None, "Human")
                .unwrap();
        let entry = ProjectService::create_entry(
            &state,
            created.project_id,
            Some(category.id),
            Some(type_def.id),
            Some("Thron".to_string()),
        )
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
        let restored_categories =
            ProjectService::list_categories(&state, restored.project_id).unwrap();
        let restored_category = restored_categories
            .iter()
            .find(|candidate| candidate.name == "Characters")
            .unwrap();
        assert_eq!(restored_category.id, category.id);
        let restored_types =
            ProjectService::list_types(&state, restored.project_id, restored_category.id).unwrap();
        assert_eq!(restored_types[0].id, type_def.id);
        assert_eq!(restored_types[0].category_id, category.id);
        let restored_entries = ProjectService::list_entries(&state, restored.project_id).unwrap();
        assert_eq!(restored_entries[0].id, entry.id);
        assert_eq!(restored_entries[0].category_id, category.id);
        assert_eq!(restored_entries[0].type_id, Some(type_def.id));

        ProjectService::update_entry_name(
            &state,
            restored.project_id,
            entry.id,
            restored_entries[0].revision,
            Some("Restored Thron".to_string()),
        )
        .unwrap();
        assert_eq!(
            ProjectService::get_entry(&state, created.project_id, entry.id)
                .unwrap()
                .authored_name
                .as_deref(),
            Some("Thron")
        );

        let original = ProjectService::get_summary(&state, created.project_id).unwrap();
        assert_eq!(original.working_name, "Tortuga");

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
            "DROP TRIGGER field_category_restrict;
             DROP TRIGGER field_type_restrict;
             DROP TRIGGER field_entry_restrict;
             DROP TABLE occurrence_entry; DROP TABLE occurrence_chapter; DROP TABLE temporal_occurrence; DROP TRIGGER occurrence_event_preserve; DROP TABLE timeline_calendar; DELETE FROM capability_def WHERE id='event'; DROP TRIGGER search_source_updated; DROP TRIGGER search_source_created; DROP TABLE search_index; DROP TABLE derived_index_state; DROP TABLE entry_alias; DROP TABLE story_link_role; DROP TABLE story_link; DROP TABLE story_role; DROP TABLE rich_document; DROP TABLE story_unit;
             DROP TRIGGER entry_materialize_capabilities; DROP TABLE spatial_node; DROP TABLE entry_capability; DROP TABLE category_capability_default; DROP TABLE type_capability_default; DROP TABLE capability_def; DROP TABLE field_projection; DROP TRIGGER projection_value_insert; DROP TRIGGER projection_value_update; DROP TABLE entry_field_presentation;
             DROP TABLE relationship_participant;
             DROP TABLE relationship_instance;
             DROP TABLE relationship_definition;
             DROP TABLE field_choice_value;
             DROP TABLE field_value;
             DROP TABLE choice_option;
             DROP TABLE field_availability;
             DROP TABLE field_definition;
             DROP TRIGGER entry_type_category_update;
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
        let recovery_db = rusqlite::Connection::open_with_flags(
            recovery,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            crate::persistence::migrations::user_version(&recovery_db).unwrap(),
            1
        );
        let (snapshot_project_id, snapshot_schema_version): (String, u32) = recovery_db
            .query_row(
                "SELECT project_id, schema_version FROM project_meta WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(snapshot_project_id, created.project_id.to_string());
        assert_eq!(snapshot_schema_version, 1);
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
            "DROP TRIGGER field_category_restrict;
             DROP TRIGGER field_type_restrict;
             DROP TRIGGER field_entry_restrict;
             DROP TABLE occurrence_entry; DROP TABLE occurrence_chapter; DROP TABLE temporal_occurrence; DROP TRIGGER occurrence_event_preserve; DROP TABLE timeline_calendar; DELETE FROM capability_def WHERE id='event'; DROP TRIGGER search_source_updated; DROP TRIGGER search_source_created; DROP TABLE search_index; DROP TABLE derived_index_state; DROP TABLE entry_alias; DROP TABLE story_link_role; DROP TABLE story_link; DROP TABLE story_role; DROP TABLE rich_document; DROP TABLE story_unit;
             DROP TRIGGER entry_materialize_capabilities; DROP TABLE spatial_node; DROP TABLE entry_capability; DROP TABLE category_capability_default; DROP TABLE type_capability_default; DROP TABLE capability_def; DROP TABLE field_projection; DROP TRIGGER projection_value_insert; DROP TRIGGER projection_value_update; DROP TABLE entry_field_presentation;
             DROP TABLE relationship_participant;
             DROP TABLE relationship_instance;
             DROP TABLE relationship_definition;
             DROP TABLE field_choice_value;
             DROP TABLE field_value;
             DROP TABLE choice_option;
             DROP TABLE field_availability;
             DROP TABLE field_definition;
             DROP TRIGGER entry_type_category_update;
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
