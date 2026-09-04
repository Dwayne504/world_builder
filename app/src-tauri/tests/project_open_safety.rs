use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::Connection;
use tempfile::tempdir;
use worldcrafter_lib::application::{AppState, ProjectService};
use worldcrafter_lib::domain::ProjectId;
use worldcrafter_lib::package::{self, layout, manifest::Manifest, PackagePaths};
use worldcrafter_lib::persistence::lock::LockInfo;
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

/// Builds a real, closed Project package so all lock scenarios run against
/// the same structure the Home screen opens.
fn make_closed_project(
    state: &AppState,
    dir: &Path,
) -> (worldcrafter_lib::application::ProjectSummary, PathBuf) {
    let created = ProjectService::create_project(state, dir, "Tortuga").unwrap();
    let package_path = PathBuf::from(&created.package_path);
    ProjectService::close_project(state, created.project_id).unwrap();
    (created, package_path)
}

/// Simulates a crashed instance: orphaned heartbeat metadata with the
/// given heartbeat age, and no OS-held advisory guard.
fn plant_orphaned_lock_metadata(paths: &PackagePaths, heartbeat_age: chrono::Duration) {
    let mut orphan = LockInfo::new(ProjectId::new());
    orphan.heartbeat_at = Utc::now() - heartbeat_age;
    fs::write(
        paths.lock_path(),
        serde_json::to_string_pretty(&orphan).unwrap(),
    )
    .unwrap();
}

fn lock_metadata(paths: &PackagePaths) -> LockInfo {
    serde_json::from_str(&fs::read_to_string(paths.lock_path()).unwrap()).unwrap()
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
    assert_eq!(err.kind(), "persistence_error");
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

/// Regression test for a native-close discard bug: a working-name edit that
/// is never submitted through `rename_project` must not be persisted by
/// `close_project`, and must not reappear when the Project is reopened. The
/// UI-level fix removes the blur-triggered save that used to let native
/// window close commit an unsubmitted draft; this test pins the backend
/// half of the contract that makes that fix sufficient: `close_project` has
/// no side effect on `working_name` when no rename was ever submitted.
#[test]
fn discarding_an_unsubmitted_rename_preserves_the_saved_working_name_after_reopen() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let created = ProjectService::create_project(&state, dir.path(), "TEST").unwrap();
    let package_path = PathBuf::from(&created.package_path);

    // Simulate the user typing "test" into the working-name draft but never
    // calling rename_project (i.e. choosing Discard instead of Save), then
    // closing the Project -- exactly the reproduction from manual testing.
    ProjectService::close_project(&state, created.project_id).unwrap();

    let reopened = ProjectService::open_project(&state, &package_path, false).unwrap();
    assert_eq!(reopened.project_id, created.project_id);
    assert_eq!(reopened.working_name, "TEST");
    assert_eq!(reopened.revision, created.revision);

    ProjectService::close_project(&state, reopened.project_id).unwrap();
}

#[test]
fn stale_orphaned_lock_metadata_reports_recovery_required_then_recovers() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let (created, package_path) = make_closed_project(&state, dir.path());
    let paths = project_paths(&created.package_path);
    plant_orphaned_lock_metadata(&paths, chrono::Duration::hours(2));

    // The specific error kind the Home screen uses to offer recovery.
    let err = ProjectService::open_project(&state, &package_path, false).unwrap_err();
    assert_eq!(err.kind(), "lock_recovery_required");
    // Refusal must not have discarded or rewritten the stale evidence.
    assert!(paths.lock_path().is_file());
    assert!(paths.db_path().is_file());

    // Explicit recovery opens the very same Project, unchanged.
    let recovered = ProjectService::open_project(&state, &package_path, true).unwrap();
    assert_eq!(recovered.project_id, created.project_id);
    assert_eq!(recovered.working_name, created.working_name);
    assert_eq!(recovered.revision, created.revision);

    // Recovery replaced the orphan with metadata owned by this session.
    let owned = lock_metadata(&paths);
    assert_eq!(owned.project_id, created.project_id);

    // Normal close removes the metadata this session owns.
    ProjectService::close_project(&state, recovered.project_id).unwrap();
    assert!(!paths.lock_path().exists());
}

#[test]
fn an_active_os_owner_cannot_be_recovered_or_stolen() {
    let state_a = AppState::default();
    let state_b = AppState::default();
    let dir = tempdir().unwrap();
    let created = ProjectService::create_project(&state_a, dir.path(), "Tortuga").unwrap();
    let package_path = PathBuf::from(&created.package_path);
    let paths = project_paths(&created.package_path);

    // Even if the heartbeat metadata looks ancient (e.g. clock skew or a
    // wedged heartbeat), the actively held OS advisory lock wins.
    let mut metadata = lock_metadata(&paths);
    metadata.heartbeat_at = Utc::now() - chrono::Duration::hours(6);
    fs::write(
        paths.lock_path(),
        serde_json::to_string_pretty(&metadata).unwrap(),
    )
    .unwrap();

    for force in [false, true] {
        let err = ProjectService::open_project(&state_b, &package_path, force).unwrap_err();
        assert_eq!(err.kind(), "lock_held");
    }
    // Neither attempt removed the owner's metadata.
    assert!(paths.lock_path().is_file());

    ProjectService::close_project(&state_a, created.project_id).unwrap();
    assert!(!paths.lock_path().exists());
}

#[test]
fn fresh_orphaned_metadata_is_not_stale_and_not_recoverable() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let (created, package_path) = make_closed_project(&state, dir.path());
    let paths = project_paths(&created.package_path);
    plant_orphaned_lock_metadata(&paths, chrono::Duration::minutes(1));

    // Too recent to call stale: normal open refuses without offering
    // recovery, and even an explicit recovery attempt is refused.
    let err = ProjectService::open_project(&state, &package_path, false).unwrap_err();
    assert_eq!(err.kind(), "lock_not_stale");
    let err = ProjectService::open_project(&state, &package_path, true).unwrap_err();
    assert_eq!(err.kind(), "lock_not_stale");
    // Fresh evidence is preserved, never auto-deleted.
    assert!(paths.lock_path().is_file());
    assert!(paths.db_path().is_file());
}

#[test]
fn corrupt_lock_metadata_fails_safely_and_is_never_deleted() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let (created, package_path) = make_closed_project(&state, dir.path());
    let paths = project_paths(&created.package_path);
    fs::write(paths.lock_path(), b"{ not valid json").unwrap();

    for force in [false, true] {
        let err = ProjectService::open_project(&state, &package_path, force).unwrap_err();
        assert_eq!(err.kind(), "lock_metadata_corrupt");
    }
    // The corrupt file is evidence for the user, not debris to sweep away.
    assert_eq!(fs::read(paths.lock_path()).unwrap(), b"{ not valid json");
    assert!(paths.db_path().is_file());
}

#[test]
fn a_failed_recovery_leaves_the_project_package_intact() {
    let state = AppState::default();
    let dir = tempdir().unwrap();
    let (created, package_path) = make_closed_project(&state, dir.path());
    let paths = project_paths(&created.package_path);

    let manifest_before = fs::read(paths.manifest_path()).unwrap();
    let db_before = fs::read(paths.db_path()).unwrap();
    fs::write(paths.lock_path(), b"{ not valid json").unwrap();

    let err = ProjectService::open_project(&state, &package_path, true).unwrap_err();
    assert_eq!(err.kind(), "lock_metadata_corrupt");

    // Nothing in the package was mutated or deleted by the failed attempt.
    assert_eq!(fs::read(paths.manifest_path()).unwrap(), manifest_before);
    assert_eq!(fs::read(paths.db_path()).unwrap(), db_before);
    assert_eq!(fs::read(paths.lock_path()).unwrap(), b"{ not valid json");
    // And a later open is not blocked by a guard we failed to release.
    fs::remove_file(paths.lock_path()).unwrap();
    let opened = ProjectService::open_project(&state, &package_path, false).unwrap();
    assert_eq!(opened.project_id, created.project_id);
    ProjectService::close_project(&state, opened.project_id).unwrap();
}
