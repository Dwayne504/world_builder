use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use tempfile::tempdir;
use worldcrafter_lib::application::{AppState, ProjectService};
use worldcrafter_lib::application_home::{RecentError, RecentProjectsStore};

fn store(path: &Path) -> RecentProjectsStore {
    RecentProjectsStore::new(path.join("recent-projects.sqlite"))
}

#[test]
fn failed_retention_transaction_preserves_the_previous_recent_list() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let recents = store(dir.path());
    let mut projects = Vec::new();
    for name in ["One", "Two", "Three"] {
        let project = ProjectService::create_project(&state, dir.path(), name).unwrap();
        recents.remember(&project).unwrap();
        projects.push(project);
    }
    let before: Vec<_> = recents
        .list()
        .unwrap()
        .into_iter()
        .map(|p| p.project_id)
        .collect();
    let connection = rusqlite::Connection::open(dir.path().join("recent-projects.sqlite")).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_retention BEFORE DELETE ON recent_project BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    let fourth = ProjectService::create_project(&state, dir.path(), "Four").unwrap();
    assert!(recents.remember(&fourth).is_err());
    drop(connection);
    let after: Vec<_> = store(dir.path())
        .list()
        .unwrap()
        .into_iter()
        .map(|p| p.project_id)
        .collect();
    assert_eq!(after, before);
    projects.push(fourth);
    for project in projects {
        ProjectService::close_project(&state, project.project_id).unwrap();
    }
}

#[test]
fn recent_projects_are_bounded_deduplicated_and_renamed_across_restart() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let recents = store(dir.path());
    let mut projects = Vec::new();
    for name in ["One", "Two", "Three", "Four"] {
        let project = ProjectService::create_project(&state, dir.path(), name).unwrap();
        recents.remember(&project).unwrap();
        projects.push(project);
    }
    assert_eq!(
        recents
            .list()
            .unwrap()
            .iter()
            .map(|p| p.working_name.as_str())
            .collect::<Vec<_>>(),
        ["Four", "Three", "Two"]
    );
    let renamed = ProjectService::rename_project(
        &state,
        projects[1].project_id,
        "Second",
        projects[1].revision,
    )
    .unwrap();
    recents.remember(&renamed).unwrap();
    let restarted = store(dir.path()).list().unwrap();
    assert_eq!(restarted.len(), 3);
    assert_eq!(restarted[0].working_name, "Second");
    assert_eq!(restarted[0].project_id, projects[1].project_id.to_string());
    assert!(restarted[0].available);
    for project in projects {
        ProjectService::close_project(&state, project.project_id).unwrap();
    }
}

#[test]
fn missing_paths_stay_repairable_and_forgetting_never_deletes_packages() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let recents = store(dir.path());
    let project = ProjectService::create_project(&state, dir.path(), "Moved").unwrap();
    recents.remember(&project).unwrap();
    ProjectService::close_project(&state, project.project_id).unwrap();
    let moved = dir.path().join("Moved-again.wcproj");
    std::fs::rename(&project.package_path, &moved).unwrap();
    assert!(!recents.list().unwrap()[0].available);
    assert!(ProjectService::open_expected_project(
        &state,
        &PathBuf::from(&project.package_path),
        false,
        Some(project.project_id)
    )
    .is_err());
    assert_eq!(recents.list().unwrap().len(), 1);
    let opened =
        ProjectService::open_expected_project(&state, &moved, false, Some(project.project_id))
            .unwrap();
    recents.remember(&opened).unwrap();
    ProjectService::close_project(&state, project.project_id).unwrap();
    assert_eq!(
        recents.path_for(project.project_id).unwrap(),
        std::fs::canonicalize(&moved).unwrap()
    );
    recents.forget(project.project_id).unwrap();
    assert!(recents.list().unwrap().is_empty());
    assert!(moved.join("data/project.sqlite").is_file());
}

#[test]
fn wrong_project_at_recent_path_is_rejected_before_lock_or_database_writes() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let a = ProjectService::create_project(&state, dir.path(), "A").unwrap();
    let b = ProjectService::create_project(&state, dir.path(), "B").unwrap();
    ProjectService::close_project(&state, a.project_id).unwrap();
    ProjectService::close_project(&state, b.project_id).unwrap();
    let path = PathBuf::from(&b.package_path);
    let db_before = std::fs::read(path.join("data/project.sqlite")).unwrap();
    let manifest_before = std::fs::read(path.join("manifest.json")).unwrap();
    let error = ProjectService::open_expected_project(&state, &path, false, Some(a.project_id))
        .unwrap_err();
    assert_eq!(error.kind(), "identity_mismatch");
    assert_eq!(
        std::fs::read(path.join("data/project.sqlite")).unwrap(),
        db_before
    );
    assert_eq!(
        std::fs::read(path.join("manifest.json")).unwrap(),
        manifest_before
    );
    assert!(!path.join("lock.json").exists());
}

#[test]
fn wrong_recent_project_never_recovers_an_interrupted_manifest() {
    for extension in ["json.next", "json.previous"] {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let a = ProjectService::create_project(&state, dir.path(), "A").unwrap();
        let b = ProjectService::create_project(&state, dir.path(), "B").unwrap();
        ProjectService::close_project(&state, a.project_id).unwrap();
        ProjectService::close_project(&state, b.project_id).unwrap();
        let path = PathBuf::from(&b.package_path);
        let manifest = path.join("manifest.json");
        let recovery = manifest.with_extension(extension);
        std::fs::rename(&manifest, &recovery).unwrap();
        let recovery_before = std::fs::read(&recovery).unwrap();
        let db_before = std::fs::read(path.join("data/project.sqlite")).unwrap();

        let error = ProjectService::open_expected_project(&state, &path, false, Some(a.project_id))
            .unwrap_err();
        assert_eq!(error.kind(), "identity_mismatch");
        assert!(!manifest.exists());
        assert_eq!(std::fs::read(&recovery).unwrap(), recovery_before);
        assert_eq!(
            std::fs::read(path.join("data/project.sqlite")).unwrap(),
            db_before
        );
        assert!(!path.join("lock.json").exists());

        // The correct shortcut still allows the existing recovery flow.
        ProjectService::open_expected_project(&state, &path, false, Some(b.project_id)).unwrap();
        assert!(manifest.is_file());
        ProjectService::close_project(&state, b.project_id).unwrap();
    }
}

#[test]
fn concurrent_app_instances_keep_distinct_recent_updates() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let a = ProjectService::create_project(&state, dir.path(), "Same name").unwrap();
    let sub = dir.path().join("other");
    std::fs::create_dir(&sub).unwrap();
    let b = ProjectService::create_project(&state, &sub, "Same name").unwrap();
    let ids = [a.project_id, b.project_id];
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = [a, b]
        .into_iter()
        .map(|summary| {
            let barrier = barrier.clone();
            let store = store(dir.path());
            thread::spawn(move || {
                barrier.wait();
                store.remember(&summary).unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let rows = store(dir.path()).list().unwrap();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].project_id, rows[1].project_id);
    for id in ids {
        ProjectService::close_project(&state, id).unwrap();
    }
}

#[test]
fn corrupt_or_newer_recent_store_is_preserved_and_manual_open_still_works() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recent-projects.sqlite");
    std::fs::write(&path, b"damaged recent list").unwrap();
    assert!(store(dir.path()).list().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"damaged recent list");
    let other = tempdir().unwrap();
    let newer_path = other.path().join("recent-projects.sqlite");
    let db = rusqlite::Connection::open(&newer_path).unwrap();
    db.pragma_update(None, "user_version", 99).unwrap();
    drop(db);
    let before = std::fs::read(&newer_path).unwrap();
    assert!(matches!(
        store(other.path()).list(),
        Err(RecentError::Unsupported(99))
    ));
    assert_eq!(std::fs::read(&newer_path).unwrap(), before);
    let state = AppState::default();
    let project = ProjectService::create_project(&state, dir.path(), "Unaffected").unwrap();
    assert!(store(dir.path()).remember(&project).is_err());
    ProjectService::close_project(&state, project.project_id).unwrap();
    ProjectService::open_project(&state, Path::new(&project.package_path), false).unwrap();
    ProjectService::close_project(&state, project.project_id).unwrap();
}
