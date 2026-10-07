use rusqlite::Connection;
use std::{path::Path, sync::Arc, thread};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        lifecycle::StructureCommand,
        navigation::*,
        story::{StoryCommand, WorkspaceState},
        timeline::TimelineCommand,
        Entry, ProjectId,
    },
    package::Manifest,
};

struct Fixture {
    dir: TempDir,
    state: Arc<AppState>,
    project: ProjectId,
    path: String,
    entry: Entry,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = Arc::new(AppState::default());
        let p = ProjectService::create_project(&state, dir.path(), "Navigation fixture").unwrap();
        let entry =
            ProjectService::create_entry(&state, p.project_id, None, None, Some("Original".into()))
                .unwrap();
        Self {
            dir,
            state,
            project: p.project_id,
            path: p.package_path,
            entry,
        }
    }
    fn target(&self) -> NavigationTarget {
        NavigationTarget {
            record_kind: NavigationKind::Entry,
            record_id: self.entry.id.to_string(),
        }
    }
    fn run(&self, c: NavigationCommand) -> NavigationSnapshot {
        ProjectService::navigation(&self.state, self.project, Some(c)).unwrap()
    }
    fn read(&self) -> NavigationSnapshot {
        ProjectService::navigation(&self.state, self.project, None).unwrap()
    }
    fn revision(&self) -> i64 {
        ProjectService::read_entry_description(&self.state, self.project, self.entry.id)
            .unwrap()
            .global_revision
    }
    fn db(&self) -> Connection {
        Connection::open(Path::new(&self.path).join("data/project.sqlite")).unwrap()
    }
}

#[test]
fn stable_pins_and_recents_resolve_renames_lifecycle_and_keep_authored_revision() {
    let f = Fixture::new();
    let revision = f.revision();
    f.run(NavigationCommand::Visit { target: f.target() });
    f.run(NavigationCommand::Pin {
        target: f.target(),
        pinned: true,
    });
    f.run(NavigationCommand::Pin {
        target: f.target(),
        pinned: true,
    });
    assert_eq!(f.revision(), revision);
    ProjectService::update_entry_name(
        &f.state,
        f.project,
        f.entry.id,
        f.entry.revision,
        Some("Renamed".into()),
    )
    .unwrap();
    for state in [
        WorkspaceState::Archived,
        WorkspaceState::Trashed,
        WorkspaceState::Active,
    ] {
        ProjectService::apply_structure(
            &f.state,
            f.project,
            f.revision(),
            StructureCommand::SetEntryState {
                id: f.entry.id,
                state,
            },
            None,
        )
        .unwrap();
        let nav = f.read();
        assert_eq!(nav.pins.len(), 1);
        assert_eq!(nav.pins[0].label, "Renamed");
        assert_eq!(nav.recents[0].target, f.target());
        assert_eq!(
            nav.pins[0].workspace_state,
            serde_json::to_value(state).unwrap().as_str().unwrap()
        );
    }
    f.run(NavigationCommand::Pin {
        target: f.target(),
        pinned: false,
    });
    assert!(f.read().pins.is_empty());
    assert_eq!(f.read().recents.len(), 1);
}

#[test]
fn bounded_recents_reorder_without_duplicates_and_do_not_evict_pins() {
    let f = Fixture::new();
    f.run(NavigationCommand::Pin {
        target: f.target(),
        pinned: true,
    });
    let mut targets = vec![];
    for _ in 0..23 {
        let e = ProjectService::create_entry(&f.state, f.project, None, None, None).unwrap();
        let target = NavigationTarget {
            record_kind: NavigationKind::Entry,
            record_id: e.id.to_string(),
        };
        f.run(NavigationCommand::Visit {
            target: target.clone(),
        });
        targets.push(target);
    }
    assert_eq!(f.read().recents.len(), 20);
    assert_eq!(f.read().recents[0].target, targets[22]);
    let result = f.run(NavigationCommand::SetRecentLimit { limit: 3 });
    assert_eq!(result.recents.len(), 3);
    assert_eq!(result.pins.len(), 1);
    let result = f.run(NavigationCommand::Visit {
        target: targets[20].clone(),
    });
    assert_eq!(result.recents.len(), 3);
    assert_eq!(result.recents[0].target, targets[20]);
    let result = f.run(NavigationCommand::ClearRecents);
    assert!(result.recents.is_empty());
    assert_eq!(result.pins.len(), 1);
}

#[test]
fn foreign_wrong_kind_missing_and_invalid_requests_leave_navigation_intact() {
    let f = Fixture::new();
    f.run(NavigationCommand::Pin {
        target: f.target(),
        pinned: true,
    });
    let other = ProjectService::create_project(&f.state, f.dir.path(), "Other").unwrap();
    let foreign =
        ProjectService::create_entry(&f.state, other.project_id, None, None, None).unwrap();
    for target in [
        NavigationTarget {
            record_kind: NavigationKind::StoryUnit,
            ..f.target()
        },
        NavigationTarget {
            record_kind: NavigationKind::Entry,
            record_id: foreign.id.to_string(),
        },
        NavigationTarget {
            record_kind: NavigationKind::Entry,
            record_id: "not-an-id".into(),
        },
    ] {
        for command in [
            NavigationCommand::Visit {
                target: target.clone(),
            },
            NavigationCommand::Pin {
                target,
                pinned: true,
            },
        ] {
            assert!(ProjectService::navigation(&f.state, f.project, Some(command)).is_err());
        }
    }
    for limit in [0, 101] {
        assert!(ProjectService::navigation(
            &f.state,
            f.project,
            Some(NavigationCommand::SetRecentLimit { limit })
        )
        .is_err());
    }
    assert_eq!(f.read().pins.len(), 1);
    assert!(f.read().recents.is_empty());
    assert!(ProjectService::navigation(&f.state, other.project_id, None)
        .unwrap()
        .pins
        .is_empty());
}

#[test]
fn chapters_occurrences_and_restore_copy_keep_internal_targets_and_order() {
    let f = Fixture::new();
    let chapter = ProjectService::apply_story(
        &f.state,
        f.project,
        f.revision(),
        StoryCommand::Create {
            title: "Chapter".into(),
        },
    )
    .unwrap();
    let timeline =
        ProjectService::apply_timeline(&f.state, f.project, f.revision(), TimelineCommand::Create)
            .unwrap();
    let targets = [
        f.target(),
        NavigationTarget {
            record_kind: NavigationKind::StoryUnit,
            record_id: chapter.chapter.id.to_string(),
        },
        NavigationTarget {
            record_kind: NavigationKind::TemporalOccurrence,
            record_id: timeline.occurrences[0].id.to_string(),
        },
    ];
    for target in &targets {
        f.run(NavigationCommand::Pin {
            target: target.clone(),
            pinned: true,
        });
        f.run(NavigationCommand::Visit {
            target: target.clone(),
        });
    }
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        &backup,
        &f.dir.path().join("copy.wcproj"),
        Some("Copy"),
    )
    .unwrap();
    assert_ne!(copy.project_id, f.project);
    let copied = ProjectService::navigation(&f.state, copy.project_id, None).unwrap();
    assert_eq!(
        copied
            .pins
            .iter()
            .map(|r| r.target.clone())
            .collect::<Vec<_>>(),
        targets
    );
    assert_eq!(
        copied
            .recents
            .iter()
            .map(|r| r.target.clone())
            .collect::<Vec<_>>(),
        targets.into_iter().rev().collect::<Vec<_>>()
    );
    f.run(NavigationCommand::ClearRecents);
    assert_eq!(
        ProjectService::navigation(&f.state, copy.project_id, None)
            .unwrap()
            .recents
            .len(),
        3
    );
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert_eq!(f.read().pins.len(), 3);
    assert!(f.read().recents.is_empty());
}

#[test]
fn concurrent_visits_are_serialized_and_do_not_stale_writing() {
    let f = Fixture::new();
    let rev = f.revision();
    let joins = (0..8)
        .map(|_| {
            let state = f.state.clone();
            let project = f.project;
            let target = f.target();
            thread::spawn(move || {
                ProjectService::navigation(
                    &state,
                    project,
                    Some(NavigationCommand::Visit { target }),
                )
                .unwrap()
            })
        })
        .collect::<Vec<_>>();
    for j in joins {
        j.join().unwrap();
    }
    assert_eq!(f.read().recents.len(), 1);
    assert_eq!(f.revision(), rev);
    ProjectService::save_entry_description(&f.state,f.project,f.entry.id,rev,None,1,serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Kept writing"}]}]})).unwrap();
}

#[test]
fn stale_navigation_rows_do_not_break_content_and_can_be_unpinned() {
    let f = Fixture::new();
    f.run(NavigationCommand::Pin {
        target: f.target(),
        pinned: true,
    });
    let missing = uuid::Uuid::now_v7().to_string();
    let db = f.db();
    db.pragma_update(None, "foreign_keys", false).unwrap();
    db.execute("UPDATE project_pin SET record_id=?1", [&missing])
        .unwrap();
    let result = f.read();
    assert_eq!(result.pins[0].workspace_state, "missing");
    assert!(ProjectService::get_entry(&f.state, f.project, f.entry.id).is_ok());
    f.run(NavigationCommand::Pin {
        target: result.pins[0].target.clone(),
        pinned: false,
    });
    assert!(f.read().pins.is_empty());
}

#[test]
fn schema_twelve_upgrade_is_atomic_and_preserves_content_and_recovery() {
    let f = Fixture::new();
    ProjectService::close_project(&f.state, f.project).unwrap();
    let db = f.db();
    db.execute_batch("DROP TABLE project_pin; DROP TABLE project_recent; DROP TABLE project_navigation_settings; PRAGMA user_version=12; UPDATE project_meta SET schema_version=12; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let manifest_path = Path::new(&f.path).join("manifest.json");
    let mut manifest = Manifest::read(&manifest_path).unwrap();
    manifest.schema_version = 12;
    manifest.write(&manifest_path).unwrap();
    assert!(ProjectService::open_project(&f.state, Path::new(&f.path), false).is_err());
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        12
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='project_pin'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(db);
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert!(f.read().pins.is_empty());
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, f.entry.id)
            .unwrap()
            .authored_name,
        Some("Original".into())
    );
}
