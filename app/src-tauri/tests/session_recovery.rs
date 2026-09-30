use std::path::Path;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::tempdir;
use worldcrafter_lib::application::{AppState, ProjectService};

#[test]
fn resume_uses_the_live_worker_latest_revision_and_unchanged_lock() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let created = ProjectService::create_project(&state, dir.path(), "Test world").unwrap();
    let path = Path::new(&created.package_path);
    let lock = std::fs::read(path.join("lock.json")).unwrap();
    let original = state.open_projects.lock().unwrap()[&created.project_id].clone();
    let renamed =
        ProjectService::rename_project(&state, created.project_id, "Revised world", 0).unwrap();
    assert_eq!(
        ProjectService::list_open_projects(&state).unwrap(),
        vec![renamed.clone()]
    );
    for force in [false, true] {
        let resumed = ProjectService::open_expected_project(
            &state,
            &path.canonicalize().unwrap(),
            force,
            Some(created.project_id),
        )
        .unwrap();
        assert_eq!(resumed, renamed);
        assert!(Arc::ptr_eq(
            &original,
            &state.open_projects.lock().unwrap()[&created.project_id]
        ));
        assert_eq!(std::fs::read(path.join("lock.json")).unwrap(), lock);
    }
    ProjectService::close_project(&state, created.project_id).unwrap();
    assert!(ProjectService::list_open_projects(&state)
        .unwrap()
        .is_empty());
    assert!(!path.join("lock.json").exists());
    let reopened = ProjectService::open_project(&AppState::default(), path, false).unwrap();
    assert_eq!(reopened.working_name, "Revised world");
}

#[test]
fn resumption_never_takes_a_different_instances_lock_or_wrong_recent_identity() {
    let dir = tempdir().unwrap();
    let owner = AppState::default();
    let stranger = AppState::default();
    let a = ProjectService::create_project(&owner, dir.path(), "A").unwrap();
    let b = ProjectService::create_project(&owner, dir.path(), "B").unwrap();
    let path = Path::new(&a.package_path);
    let lock = std::fs::read(path.join("lock.json")).unwrap();
    for force in [false, true] {
        assert_eq!(
            ProjectService::open_project(&stranger, path, force)
                .unwrap_err()
                .kind(),
            "lock_held"
        );
    }
    assert_eq!(
        ProjectService::open_expected_project(&owner, path, false, Some(b.project_id))
            .unwrap_err()
            .kind(),
        "identity_mismatch"
    );
    assert!(ProjectService::list_open_projects(&stranger)
        .unwrap()
        .is_empty());
    assert_eq!(ProjectService::list_open_projects(&owner).unwrap().len(), 2);
    assert_eq!(std::fs::read(path.join("lock.json")).unwrap(), lock);
    for id in [a.project_id, b.project_id] {
        ProjectService::close_project(&owner, id).unwrap();
    }
}

#[test]
fn a_copied_package_with_the_same_id_cannot_replace_a_live_worker() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let created = ProjectService::create_project(&state, dir.path(), "Original fixture").unwrap();
    ProjectService::close_project(&state, created.project_id).unwrap();
    let original = Path::new(&created.package_path);
    let duplicate = dir.path().join("duplicate.wcproj");
    for part in ["data", "assets", "staging"] {
        std::fs::create_dir_all(duplicate.join(part)).unwrap();
    }
    for part in ["manifest.json", "data/project.sqlite"] {
        std::fs::copy(original.join(part), duplicate.join(part)).unwrap();
    }
    let db_before = std::fs::read(duplicate.join("data/project.sqlite")).unwrap();
    ProjectService::open_project(&state, original, false).unwrap();
    assert_eq!(
        ProjectService::open_project(&state, &duplicate, false)
            .unwrap_err()
            .kind(),
        "duplicate_open_project"
    );
    assert!(!duplicate.join("lock.json").exists());
    assert_eq!(
        std::fs::read(duplicate.join("data/project.sqlite")).unwrap(),
        db_before
    );
    let resumed = ProjectService::list_open_projects(&state).unwrap();
    assert_eq!(resumed.len(), 1);
    assert_eq!(resumed[0].package_path, created.package_path);
    ProjectService::rename_project(&state, created.project_id, "Still editable", 0).unwrap();
    ProjectService::close_project(&state, created.project_id).unwrap();
}

#[test]
fn concurrent_open_requests_share_one_worker_and_one_lock() {
    let dir = tempdir().unwrap();
    let state = Arc::new(AppState::default());
    let created = ProjectService::create_project(&state, dir.path(), "Concurrent fixture").unwrap();
    ProjectService::close_project(&state, created.project_id).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let state = state.clone();
            let barrier = barrier.clone();
            let path = created.package_path.clone();
            thread::spawn(move || {
                barrier.wait();
                ProjectService::open_project(&state, Path::new(&path), false).unwrap()
            })
        })
        .collect();
    for handle in handles {
        assert_eq!(handle.join().unwrap().project_id, created.project_id);
    }
    assert_eq!(ProjectService::list_open_projects(&state).unwrap().len(), 1);
    ProjectService::close_project(&state, created.project_id).unwrap();
    assert!(!Path::new(&created.package_path).join("lock.json").exists());
}
