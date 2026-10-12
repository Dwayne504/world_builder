use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    domain::{
        story::{StoryCommand, WorkspaceState},
        structure::{ChapterId, OccurrenceId},
        timeline::*,
        EntryId, ProjectId,
    },
    persistence::{worker::InitialProjectMeta, PersistenceError, ProjectDbWorker},
};

struct Fixture {
    worker: ProjectDbWorker,
    dir: TempDir,
    project: ProjectId,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let project = ProjectId::new();
        let worker = ProjectDbWorker::spawn(
            dir.path().join("project.sqlite"),
            project,
            Some(InitialProjectMeta {
                working_name: "Timeline tests".into(),
                format_version: 1,
            }),
        )
        .unwrap();
        Self {
            worker,
            dir,
            project,
        }
    }
    fn apply(&self, command: TimelineCommand) -> TimelineSnapshot {
        self.worker
            .apply_timeline(
                self.worker.read_timeline().unwrap().global_revision,
                command,
            )
            .unwrap()
    }
    fn create(&self) -> OccurrenceId {
        let old = self.worker.read_timeline().unwrap();
        self.apply(TimelineCommand::Create)
            .occurrences
            .iter()
            .find(|o| !old.occurrences.iter().any(|p| p.id == o.id))
            .unwrap()
            .id
    }
    fn entry(&self, name: &str) -> EntryId {
        let id = EntryId::new();
        self.worker
            .create_entry(id, None, None, Some(name.into()))
            .unwrap();
        id
    }
    fn chapter(&self) -> ChapterId {
        self.worker
            .apply_story(
                self.worker.read_meta().unwrap().last_committed_revision,
                StoryCommand::Create {
                    title: "Later in reading order".into(),
                },
            )
            .unwrap()
            .chapter
            .id
    }
    fn db(&self) -> Connection {
        let db = Connection::open(self.dir.path().join("project.sqlite")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db
    }
}
fn calendar() -> Calendar {
    Calendar {
        name: "Orbit".into(),
        era_label: "AL".into(),
        months: vec![
            CalendarMonth {
                name: "Dawn".into(),
                days: 40,
            },
            CalendarMonth {
                name: "Dusk".into(),
                days: 8,
            },
        ],
    }
}
fn draft() -> OccurrenceDraft {
    OccurrenceDraft {
        title: String::new(),
        notes: String::new(),
        date: None,
        event_entry_id: None,
        entry_ids: vec![],
        chapter_ids: vec![],
    }
}

#[test]
fn stubs_are_not_entries_and_saved_notes_survive_reopen() {
    let f = Fixture::new();
    let id = f.create();
    let other = f.create();
    assert_ne!(id, other);
    let saved = f.apply(TimelineCommand::Save {
        id,
        draft: OccurrenceDraft {
            notes: "An unfinished idea".into(),
            ..draft()
        },
    });
    assert!(f.worker.list_entries().unwrap().is_empty());
    f.worker.shutdown().unwrap();
    let reopened =
        ProjectDbWorker::spawn(f.dir.path().join("project.sqlite"), f.project, None).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.read_timeline().unwrap()).unwrap(),
        serde_json::to_value(saved).unwrap()
    );
    reopened.shutdown().unwrap();
}
#[test]
fn dates_sort_in_fictional_order_and_do_not_reorder_chapters_or_rewrite_writing() {
    let f = Fixture::new();
    let c = f.chapter();
    let original = f.worker.read_chapter(c).unwrap();
    f.apply(TimelineCommand::ConfigureCalendar {
        calendar: calendar(),
    });
    let later = f.create();
    let early = f.create();
    let undated = f.create();
    for (id, year, month, day) in [(later, 1, 1, 1), (early, 0, 2, 8)] {
        f.apply(TimelineCommand::Save {
            id,
            draft: OccurrenceDraft {
                date: Some(FictionalDate { year, month, day }),
                chapter_ids: vec![c],
                ..draft()
            },
        });
    }
    let timeline = f.worker.read_timeline().unwrap();
    assert_eq!(
        timeline
            .occurrences
            .iter()
            .map(|o| o.id)
            .collect::<Vec<_>>(),
        vec![early, later, undated]
    );
    let chapter = f.worker.read_chapter(c).unwrap();
    assert_eq!(
        serde_json::to_value(chapter.chapter).unwrap(),
        serde_json::to_value(original.chapter).unwrap()
    );
    assert_eq!(
        serde_json::to_value(chapter.documents).unwrap(),
        serde_json::to_value(original.documents).unwrap()
    );
    assert!(
        chapter.links.is_empty(),
        "Chronology does not invent Story links or Roles"
    );
}
#[test]
fn invalid_dates_and_calendar_changes_fail_without_losing_saved_data() {
    let f = Fixture::new();
    let id = f.create();
    let mut d = draft();
    d.date = Some(FictionalDate {
        year: 0,
        month: 1,
        day: 1,
    });
    let rev = f.worker.read_timeline().unwrap().global_revision;
    assert!(f
        .worker
        .apply_timeline(
            rev,
            TimelineCommand::Save {
                id,
                draft: d.clone()
            }
        )
        .is_err());
    f.apply(TimelineCommand::ConfigureCalendar {
        calendar: calendar(),
    });
    f.apply(TimelineCommand::Save {
        id,
        draft: d.clone(),
    });
    let mut invalid = d.clone();
    invalid.date.as_mut().unwrap().day = 41;
    let before = f.worker.read_timeline().unwrap();
    assert!(f
        .worker
        .apply_timeline(
            before.global_revision,
            TimelineCommand::Save { id, draft: invalid }
        )
        .is_err());
    assert_eq!(
        f.worker.read_timeline().unwrap().global_revision,
        before.global_revision
    );
    f.apply(TimelineCommand::SetState {
        id,
        state: WorkspaceState::Trashed,
    });
    let mut changed = calendar();
    changed.months[0].days = 50;
    assert!(f
        .worker
        .apply_timeline(
            f.worker.read_timeline().unwrap().global_revision,
            TimelineCommand::ConfigureCalendar { calendar: changed }
        )
        .is_err());
    let mut renamed = calendar();
    renamed.months[0].name = "First light".into();
    let after = f.apply(TimelineCommand::ConfigureCalendar { calendar: renamed });
    assert_eq!(after.occurrences[0].date, d.date);
    f.apply(TimelineCommand::SetState {
        id,
        state: WorkspaceState::Active,
    });
    f.apply(TimelineCommand::Save { id, draft: draft() });
    let mut changed = calendar();
    changed.months[0].days = 50;
    f.apply(TimelineCommand::ConfigureCalendar { calendar: changed });
}
#[test]
fn event_pages_and_context_links_are_canonical_project_scoped_and_reversible() {
    let f = Fixture::new();
    let event = f.entry("A major battle");
    let person = f.entry("Traveller");
    let chapter = f.chapter();
    let id = f.create();
    let d = OccurrenceDraft {
        event_entry_id: Some(event),
        entry_ids: vec![person, person],
        chapter_ids: vec![chapter, chapter],
        ..draft()
    };
    let saved = f.apply(TimelineCommand::Save {
        id,
        draft: d.clone(),
    });
    assert_eq!(saved.occurrences[0].entries.len(), 1);
    assert_eq!(saved.occurrences[0].chapters.len(), 1);
    assert_eq!(f.worker.list_entries().unwrap().len(), 2);
    assert!(f
        .db()
        .prepare("SELECT 1 FROM entry_capability WHERE entry_id=?1 AND capability_id='event'")
        .unwrap()
        .exists([event.to_string()])
        .unwrap());
    f.worker
        .update_entry_name(event, 1, Some("Renamed battle".into()))
        .unwrap();
    assert_eq!(
        f.worker.read_timeline().unwrap().occurrences[0]
            .event_entry
            .as_ref()
            .unwrap()
            .label,
        "Renamed battle"
    );
    for state in [
        WorkspaceState::Archived,
        WorkspaceState::Trashed,
        WorkspaceState::Active,
    ] {
        let s = f.apply(TimelineCommand::SetState { id, state });
        assert_eq!(s.occurrences[0].entries[0].id, person.to_string());
        assert_eq!(s.occurrences[0].chapters[0].id, chapter.to_string());
    }
    assert_eq!(
        f.worker
            .read_chapter(chapter)
            .unwrap()
            .chapter
            .workspace_state,
        "active"
    );
    assert!(f
        .db()
        .execute("DELETE FROM entry WHERE id=?1", [event.to_string()])
        .is_err());
    let bad = OccurrenceDraft {
        entry_ids: vec![EntryId::new()],
        event_entry_id: Some(f.entry("Not yet an event")),
        ..draft()
    };
    let rev = f.worker.read_timeline().unwrap().global_revision;
    assert!(f
        .worker
        .apply_timeline(
            rev,
            TimelineCommand::Save {
                id,
                draft: bad.clone()
            }
        )
        .is_err());
    assert!(!f
        .db()
        .prepare("SELECT 1 FROM entry_capability WHERE entry_id=?1 AND capability_id='event'")
        .unwrap()
        .exists([bad.event_entry_id.unwrap().to_string()])
        .unwrap());
    assert_eq!(
        f.worker.read_timeline().unwrap().occurrences[0].entries[0].id,
        person.to_string()
    );
    f.apply(TimelineCommand::Save { id, draft: draft() });
    assert!(f.worker.read_timeline().unwrap().occurrences[0]
        .entries
        .is_empty());
    assert_eq!(f.worker.list_entries().unwrap().len(), 3);
}
#[test]
fn stale_concurrent_edits_cannot_overwrite_each_other() {
    let f = Fixture::new();
    let id = f.create();
    let rev = f.worker.read_timeline().unwrap().global_revision;
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            f.worker.apply_timeline(
                rev,
                TimelineCommand::Save {
                    id,
                    draft: OccurrenceDraft {
                        notes: "A".into(),
                        ..draft()
                    },
                },
            )
        });
        let b = scope.spawn(|| {
            f.worker.apply_timeline(
                rev,
                TimelineCommand::Save {
                    id,
                    draft: OccurrenceDraft {
                        notes: "B".into(),
                        ..draft()
                    },
                },
            )
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(PersistenceError::StaleRevision { .. })))
            .count(),
        1
    );
    assert_eq!(f.worker.read_timeline().unwrap().global_revision, rev + 1);
}
#[test]
fn corrupt_and_newer_calendars_are_not_replaced_by_any_timeline_command() {
    use worldcrafter_lib::domain::search::SearchRequest;
    for (version, json) in [
        (2, serde_json::to_string(&calendar()).unwrap()),
        (1, "{}".into()),
    ] {
        let f = Fixture::new();
        f.apply(TimelineCommand::ConfigureCalendar {
            calendar: calendar(),
        });
        let id = f.create();
        f.apply(TimelineCommand::Save {
            id,
            draft: OccurrenceDraft {
                title: "Preserved occurrence".into(),
                date: Some(FictionalDate {
                    year: 1,
                    month: 1,
                    day: 1,
                }),
                ..draft()
            },
        });
        let rev = f.worker.read_meta().unwrap().last_committed_revision;
        f.db()
            .execute(
                "UPDATE timeline_calendar SET schema_version=?1,config_json=?2",
                params![version, json],
            )
            .unwrap();
        assert!(f.worker.read_timeline().is_err());
        // Unrelated discovery still works; search does not interpret fictional dates.
        let results = f
            .worker
            .search_project(SearchRequest {
                query: "preserved".into(),
                include_inactive: false,
                limit_per_group: 10,
                entry_id: None,
                structured_kind: None,
                text_area: None,
                story_role_id: None,
            })
            .unwrap();
        assert_eq!(
            results
                .groups
                .iter()
                .find(|g| g.kind == "timeline")
                .unwrap()
                .total,
            1
        );
        assert!(f
            .worker
            .apply_timeline(
                rev,
                TimelineCommand::ConfigureCalendar {
                    calendar: calendar()
                }
            )
            .is_err());
        assert!(f
            .worker
            .apply_timeline(rev, TimelineCommand::Create)
            .is_err());
        assert_eq!(
            f.db()
                .query_row("SELECT config_json FROM timeline_calendar", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            json
        );
        assert_eq!(f.worker.read_meta().unwrap().last_committed_revision, rev);
    }
}
#[test]
fn failed_link_publication_rolls_back_all_parts() {
    let f = Fixture::new();
    let entry = f.entry("Context");
    let id = f.create();
    let rev = f.worker.read_timeline().unwrap().global_revision;
    f.db().execute_batch("CREATE TRIGGER fail_link BEFORE INSERT ON occurrence_entry BEGIN SELECT RAISE(ABORT,'injected link failure'); END;").unwrap();
    assert!(f
        .worker
        .apply_timeline(
            rev,
            TimelineCommand::Save {
                id,
                draft: OccurrenceDraft {
                    title: "Must roll back".into(),
                    event_entry_id: Some(entry),
                    entry_ids: vec![entry],
                    ..draft()
                }
            }
        )
        .is_err());
    let after = f.worker.read_timeline().unwrap();
    assert_eq!(after.global_revision, rev);
    assert!(after.occurrences[0].title.is_empty());
    assert!(after.occurrences[0].event_entry.is_none());
    assert!(!f
        .db()
        .prepare("SELECT 1 FROM entry_capability WHERE capability_id='event'")
        .unwrap()
        .exists([])
        .unwrap());
}
#[test]
fn global_search_and_entry_scope_use_explicit_links_only() {
    use worldcrafter_lib::domain::search::{SearchRequest, SearchTarget};
    let f = Fixture::new();
    let entry = f.entry("Traveller");
    let id = f.create();
    f.apply(TimelineCommand::Save {
        id,
        draft: OccurrenceDraft {
            title: "Arrival".into(),
            notes: "Traveller arrived here".into(),
            ..draft()
        },
    });
    let mut request = SearchRequest {
        query: "arrival".into(),
        include_inactive: false,
        limit_per_group: 10,
        entry_id: None,
        structured_kind: None,
        text_area: None,
        story_role_id: None,
    };
    for _ in 0..2 {
        let results = f.worker.search_project(request.clone()).unwrap();
        let hit = &results
            .groups
            .iter()
            .find(|g| g.kind == "timeline")
            .unwrap()
            .hits[0];
        assert!(
            matches!(&hit.target,SearchTarget::Occurrence{occurrence_id} if occurrence_id==&id.to_string())
        );
    }
    request.entry_id = Some(entry);
    assert!(f
        .worker
        .search_project(request.clone())
        .unwrap()
        .groups
        .iter()
        .all(|g| g.hits.is_empty()));
    f.apply(TimelineCommand::Save {
        id,
        draft: OccurrenceDraft {
            title: "Arrival".into(),
            entry_ids: vec![entry],
            ..draft()
        },
    });
    assert_eq!(
        f.worker
            .search_project(request)
            .unwrap()
            .groups
            .iter()
            .find(|g| g.kind == "timeline")
            .unwrap()
            .total,
        1
    );
}
#[test]
fn timeline_backup_restores_as_an_independent_project_with_internal_ids_intact() {
    use worldcrafter_lib::application::{AppState, ProjectService};
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let project = ProjectService::create_project(&state, dir.path(), "Chronicle").unwrap();
    let entry = ProjectService::create_entry(
        &state,
        project.project_id,
        None,
        None,
        Some("Battle".into()),
    )
    .unwrap();
    let s = ProjectService::apply_timeline(
        &state,
        project.project_id,
        entry.global_revision,
        TimelineCommand::Create,
    )
    .unwrap();
    let id = s.occurrences[0].id;
    let s = ProjectService::apply_timeline(
        &state,
        project.project_id,
        s.global_revision,
        TimelineCommand::ConfigureCalendar {
            calendar: calendar(),
        },
    )
    .unwrap();
    let s = ProjectService::apply_timeline(
        &state,
        project.project_id,
        s.global_revision,
        TimelineCommand::Save {
            id,
            draft: OccurrenceDraft {
                event_entry_id: Some(entry.id),
                date: Some(FictionalDate {
                    year: -4,
                    month: 2,
                    day: 8,
                }),
                notes: "Kept in the snapshot".into(),
                ..draft()
            },
        },
    )
    .unwrap();
    let backup =
        ProjectService::create_backup(&state, project.project_id, &dir.path().join("backups"))
            .unwrap();
    let copy =
        ProjectService::restore_backup_as_copy(&state, &backup, dir.path(), Some("Chronicle copy"))
            .unwrap();
    assert_ne!(project.project_id, copy.project_id);
    let restored = ProjectService::read_timeline(&state, copy.project_id).unwrap();
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        serde_json::to_value(&restored).unwrap()
    );
    ProjectService::apply_timeline(
        &state,
        copy.project_id,
        restored.global_revision,
        TimelineCommand::SetState {
            id,
            state: WorkspaceState::Trashed,
        },
    )
    .unwrap();
    assert_eq!(
        ProjectService::read_timeline(&state, project.project_id)
            .unwrap()
            .occurrences[0]
            .workspace_state,
        "active"
    );
    ProjectService::close_project(&state, project.project_id).unwrap();
    ProjectService::close_project(&state, copy.project_id).unwrap();
}

#[test]
fn acknowledged_occurrence_survives_abrupt_exit_and_unfinished_edit_rolls_back() {
    let f = Fixture::new();
    let id = f.create();
    let path = f.dir.path().join("project.sqlite");
    f.worker.shutdown().unwrap();
    for mode in ["committed", "unfinished"] {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "timeline_exit_helper", "--ignored"])
            .env("WC_TIMELINE_TEST_DB", &path)
            .env("WC_TIMELINE_TEST_PROJECT", f.project.to_string())
            .env("WC_TIMELINE_TEST_ID", id.to_string())
            .env("WC_TIMELINE_TEST_MODE", mode)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        let worker = ProjectDbWorker::spawn(path.clone(), f.project, None).unwrap();
        assert_eq!(
            worker.read_timeline().unwrap().occurrences[0].notes,
            "Acknowledged"
        );
        worker.shutdown().unwrap();
    }
}
#[test]
#[ignore = "subprocess helper invoked by the abrupt-exit integration test"]
fn timeline_exit_helper() {
    let path = std::path::PathBuf::from(std::env::var("WC_TIMELINE_TEST_DB").unwrap());
    if std::env::var("WC_TIMELINE_TEST_MODE").unwrap() == "committed" {
        let worker = ProjectDbWorker::spawn(
            path,
            ProjectId::parse(&std::env::var("WC_TIMELINE_TEST_PROJECT").unwrap()).unwrap(),
            None,
        )
        .unwrap();
        worker
            .apply_timeline(
                worker.read_timeline().unwrap().global_revision,
                TimelineCommand::Save {
                    id: OccurrenceId::parse(&std::env::var("WC_TIMELINE_TEST_ID").unwrap())
                        .unwrap(),
                    draft: OccurrenceDraft {
                        notes: "Acknowledged".into(),
                        ..draft()
                    },
                },
            )
            .unwrap();
    } else {
        let db = Connection::open(path).unwrap();
        db.execute_batch("BEGIN IMMEDIATE; UPDATE temporal_occurrence SET notes='Unfinished'")
            .unwrap();
        std::process::exit(0);
    }
    std::process::exit(0);
}

#[test]
fn schema_ten_upgrade_preserves_references_and_recovers_after_injected_failure() {
    use worldcrafter_lib::{
        application::{AppState, ProjectService},
        package::manifest::Manifest,
        persistence::migrations::CURRENT_SCHEMA_VERSION,
    };
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let project = ProjectService::create_project(&state, dir.path(), "Before chronology").unwrap();
    let entry = ProjectService::create_entry(
        &state,
        project.project_id,
        None,
        None,
        Some("Existing traveller".into()),
    )
    .unwrap();
    let chapter = ProjectService::apply_story(
        &state,
        project.project_id,
        entry.global_revision,
        StoryCommand::Create {
            title: "Existing writing".into(),
        },
    )
    .unwrap();
    let chapter = ProjectService::apply_story(
        &state,
        project.project_id,
        chapter.global_revision,
        StoryCommand::SetLink {
            chapter_id: chapter.chapter.id,
            entry_id: entry.id,
            role_ids: vec![chapter.roles[0].id.clone()],
        },
    )
    .unwrap();
    ProjectService::close_project(&state, project.project_id).unwrap();
    let root = std::path::Path::new(&project.package_path);
    let db = Connection::open(root.join("data/project.sqlite")).unwrap();
    // Construct a genuine schema-10 registry, preserving the populated reference graph.
    db.execute_batch("PRAGMA foreign_keys=OFF; PRAGMA legacy_alter_table=ON;
      DROP TRIGGER rich_document_owner_insert; DROP TRIGGER rich_document_owner_update; DROP TRIGGER rich_document_keep_entry; DROP TRIGGER rich_document_keep_chapter; DROP TABLE occurrence_entry; DROP TABLE occurrence_chapter; DROP TABLE temporal_occurrence; DROP TRIGGER occurrence_event_preserve; DROP TABLE timeline_calendar;
      ALTER TABLE record_identity RENAME TO new_identity;
      CREATE TABLE record_identity(record_id TEXT PRIMARY KEY,kind TEXT NOT NULL CHECK(kind IN ('entry','story_unit','relationship_instance')),workspace_state TEXT NOT NULL CHECK(workspace_state IN ('active','archived','trashed')),lifecycle_changed_at TEXT NOT NULL,created_at TEXT NOT NULL);
      INSERT INTO record_identity SELECT * FROM new_identity; DROP TABLE new_identity;
      ALTER TABLE capability_def RENAME TO new_capabilities;
      CREATE TABLE capability_def(id TEXT PRIMARY KEY CHECK(id IN ('base','spatial')));
      INSERT INTO capability_def SELECT id FROM new_capabilities WHERE id <> 'event'; DROP TABLE new_capabilities;
      PRAGMA user_version=10; UPDATE project_meta SET schema_version=10;
      CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;").unwrap();
    let mut manifest = Manifest::read(&root.join("manifest.json")).unwrap();
    manifest.schema_version = 10;
    manifest.write(&root.join("manifest.json")).unwrap();
    assert!(ProjectService::open_project(&state, root, false).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        10
    );
    assert!(db.prepare("SELECT * FROM timeline_calendar").is_err());
    assert_eq!(
        db.query_row(
            "SELECT sql FROM sqlite_master WHERE name='record_identity'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap()
        .matches("temporal_occurrence")
        .count(),
        0
    );
    db.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(db);
    let opened = ProjectService::open_project(&state, root, false).unwrap();
    assert_eq!(opened.schema_version, CURRENT_SCHEMA_VERSION);
    let after =
        ProjectService::read_chapter(&state, project.project_id, chapter.chapter.id).unwrap();
    assert_eq!(
        serde_json::to_value(chapter).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    let snapshot = ProjectService::read_timeline(&state, project.project_id).unwrap();
    assert!(snapshot.calendar.is_none());
    assert!(snapshot.occurrences.is_empty());
    let db = Connection::open(root.join("data/project.sqlite")).unwrap();
    assert!(!db
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    assert!(!db
        .prepare("SELECT 1 FROM sqlite_master WHERE sql LIKE '%old_timeline_%'")
        .unwrap()
        .exists([])
        .unwrap());
    let recovery = Connection::open_with_flags(
        dir.path()
            .join(".worldcrafter-migration-recovery")
            .join(project.project_id.to_string())
            .join("schema-v10.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        10
    );
    assert_eq!(
        recovery
            .query_row(
                "SELECT authored_name FROM entry WHERE id=?1",
                [entry.id.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "Existing traveller"
    );
    ProjectService::close_project(&state, project.project_id).unwrap();
}
