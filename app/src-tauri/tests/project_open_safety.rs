use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::Connection;
use tempfile::tempdir;
use worldcrafter_lib::application::{AppState, ProjectService};
use worldcrafter_lib::domain::ProjectId;
use worldcrafter_lib::package::{self, layout, manifest::Manifest, PackagePaths};
use worldcrafter_lib::persistence::migrations;

fn project_paths(summary_path: &str) -> PackagePaths {
    PackagePaths::new(PathBuf::from(summary_path))
}

fn relocate_manifest_for_recovery(paths: &PackagePaths, extension: &str) {
    let manifest_path = paths.manifest_path();
    let recovery_path = manifest_path.with_extension(extension);
    fs::rename(&manifest_path, &recovery_path).unwrap();
    assert!(!manifest_path.exists());
    assert!(recovery_path.is_file());
}

fn make_existing_package(root: &Path, project_id: ProjectId) -> PackagePaths {
    let paths = layout::create_skeleton(root).unwrap();
    let manifest = Manifest::new(
        project_id,
        package::FORMAT_VERSION,
        migrations::CURRENT_SCHEMA_VERSION,
        "Tortuga",
    );
    manifest.write(&paths.manifest_path()).unwrap();
    paths
}

#[test]
fn open_project_recovers_manifest_from_synced_successor_file() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
    let package_path = PathBuf::from(&created.package_path);
    ProjectService::close_project(&state, created.project_id).unwrap();

    let paths = project_paths(&created.package_path);
    relocate_manifest_for_recovery(&paths, "json.next");

    let reopened = ProjectService::open_project(&state, &package_path, false).unwrap();
    assert_eq!(reopened.project_id, created.project_id);
    assert!(paths.manifest_path().is_file());
    assert!(!paths.manifest_path().with_extension("json.next").exists());

    ProjectService::close_project(&state, reopened.project_id).unwrap();
}

#[test]
fn open_project_recovers_manifest_from_prior_manifest_file() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let created = ProjectService::create_project(&state, dir.path(), "Tortuga").unwrap();
    let package_path = PathBuf::from(&created.package_path);
    ProjectService::close_project(&state, created.project_id).unwrap();

    let paths = project_paths(&created.package_path);
    relocate_manifest_for_recovery(&paths, "json.previous");

    let reopened = ProjectService::open_project(&state, &package_path, false).unwrap();
    assert_eq!(reopened.project_id, created.project_id);
    assert!(paths.manifest_path().is_file());
    assert!(!paths
        .manifest_path()
        .with_extension("json.previous")
        .exists());

    ProjectService::close_project(&state, reopened.project_id).unwrap();
}

#[test]
fn open_project_preflights_existing_db_before_lock_or_pragmas() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let project_id = ProjectId::new();
    let package_path = dir.path().join("Tortuga.wcproj");
    let paths = make_existing_package(&package_path, project_id);
    let conn = Connection::open(paths.db_path()).unwrap();
    let journal_mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode.to_lowercase(), "delete");
    conn.execute_batch(
        "CREATE TABLE project_meta (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            project_id TEXT NOT NULL UNIQUE,
            format_version INTEGER NOT NULL,
            schema_version INTEGER NOT NULL,
            working_name TEXT NOT NULL,
            last_committed_revision INTEGER NOT NULL DEFAULT 0,
            restored_from_project_id TEXT NULL,
            restored_from_backup_id TEXT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );",
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 0).unwrap();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO project_meta (
            id, project_id, format_version, schema_version, working_name,
            last_committed_revision, restored_from_project_id, restored_from_backup_id,
            created_at, updated_at
        ) VALUES (1, ?1, ?2, 0, ?3, 0, NULL, NULL, ?4, ?4)",
        rusqlite::params![
            project_id.to_string(),
            package::FORMAT_VERSION,
            "Tortuga",
            now,
        ],
    )
    .unwrap();
    drop(conn);

    let err = ProjectService::open_project(&state, &package_path, false).unwrap_err();
    assert_eq!(err.kind(), "migration_required");
    assert!(!paths.lock_path().exists());
    assert!(!paths.lock_path().with_extension("guard").exists());

    let conn = Connection::open(paths.db_path()).unwrap();
    let journal_mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    let user_version = migrations::user_version(&conn).unwrap();
    let schema_version: i64 = conn
        .query_row(
            "SELECT schema_version FROM project_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(journal_mode.to_lowercase(), "delete");
    assert_eq!(user_version, 0);
    assert_eq!(schema_version, 0);
}
