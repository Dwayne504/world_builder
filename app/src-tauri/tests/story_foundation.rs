use rusqlite::{params, Connection};
use serde_json::json;
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    domain::{story::*, structure::ChapterId, EntryId, ProjectId},
    persistence::{worker::InitialProjectMeta, ProjectDbWorker},
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
                working_name: "Story test".into(),
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
    fn db(&self) -> Connection {
        let db = Connection::open(self.dir.path().join("project.sqlite")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db
    }
    fn apply(&self, command: StoryCommand) -> ChapterSnapshot {
        self.worker
            .apply_story(self.worker.read_story().unwrap().global_revision, command)
            .unwrap()
    }
    fn chapter(&self, title: &str) -> ChapterSnapshot {
        self.apply(StoryCommand::Create {
            title: title.into(),
        })
    }
    fn entry(&self, name: &str) -> EntryId {
        let id = EntryId::new();
        self.worker
            .create_entry(id, None, None, Some(name.into()))
            .unwrap();
        id
    }
}
fn edit(area: DocumentArea, text: &str) -> DocumentEdit {
    DocumentEdit {
        area,
        schema_version: 1,
        content: json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]}),
    }
}
#[test]
fn loose_chapters_have_distinct_identity_and_independent_writing_areas() {
    let f = Fixture::new();
    let a = f.chapter("");
    let b = f.chapter("");
    assert_ne!(a.chapter.id, b.chapter.id);
    assert_eq!(a.documents.len(), 3);
    assert_eq!(f.worker.list_entries().unwrap().len(), 0);
    let written = f.apply(StoryCommand::Save {
        chapter_id: a.chapter.id,
        title: Some("The First Step".into()),
        documents: vec![
            edit(DocumentArea::Manuscript, "Thron enters the Temple."),
            edit(DocumentArea::Plan, "Find the Blade."),
            edit(DocumentArea::Notes, "Ask why."),
        ],
    });
    assert_eq!(
        written
            .documents
            .iter()
            .find(|d| d.area == DocumentArea::Manuscript)
            .unwrap()
            .word_count,
        4
    );
    let id = a.chapter.id;
    let path = f.dir.path().join("project.sqlite");
    f.worker.shutdown().unwrap();
    let reopened = ProjectDbWorker::spawn(path, f.project, None)
        .unwrap()
        .read_chapter(id)
        .unwrap();
    assert_eq!(reopened.chapter.title, "The First Step");
    assert!(reopened
        .documents
        .iter()
        .any(|d| d.area == DocumentArea::Notes && d.plain_text.contains("Ask why.")));
    assert_eq!(
        reopened.documents.iter().map(|d| &d.id).collect::<Vec<_>>(),
        written.documents.iter().map(|d| &d.id).collect::<Vec<_>>()
    );
}
#[test]
fn links_accept_zero_or_multiple_roles_without_duplicates_and_rename_does_not_touch_prose() {
    let f = Fixture::new();
    let c = f.chapter("The First Step");
    let thron = f.entry("Thron");
    f.apply(StoryCommand::Save {
        chapter_id: c.chapter.id,
        title: None,
        documents: vec![edit(DocumentArea::Manuscript, "Thron holds the Blade.")],
    });
    let linked = f.apply(StoryCommand::SetLink {
        chapter_id: c.chapter.id,
        entry_id: thron,
        role_ids: vec![],
    });
    assert!(linked.links[0].roles.is_empty());
    let link_id = linked.links[0].id.clone();
    let role_ids = linked
        .roles
        .iter()
        .take(2)
        .map(|r| r.id.clone())
        .collect::<Vec<_>>();
    let linked = f.apply(StoryCommand::SetLink {
        chapter_id: c.chapter.id,
        entry_id: thron,
        role_ids: role_ids.clone(),
    });
    assert_eq!(linked.links.len(), 1);
    assert_eq!(linked.links[0].id, link_id);
    assert_eq!(linked.links[0].roles.len(), 2);
    let usage = f.worker.story_usage(thron).unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].chapter.id, c.chapter.id);
    assert_eq!(usage[0].roles.len(), 2);
    f.worker
        .update_entry_name(thron, 1, Some("Emperor".into()))
        .unwrap();
    let changed = f.worker.read_chapter(c.chapter.id).unwrap();
    assert_eq!(changed.links[0].label, "Emperor");
    assert!(changed
        .documents
        .iter()
        .any(|d| d.plain_text.contains("Thron holds")));
    f.apply(StoryCommand::SetState {
        chapter_id: c.chapter.id,
        state: WorkspaceState::Trashed,
    });
    assert_eq!(
        f.worker.story_usage(thron).unwrap()[0]
            .chapter
            .workspace_state,
        "trashed"
    );
    assert!(f
        .worker
        .apply_story(
            f.worker.read_story().unwrap().global_revision,
            StoryCommand::Save {
                chapter_id: c.chapter.id,
                title: Some("bad".into()),
                documents: vec![]
            }
        )
        .is_err());
    let restored = f.apply(StoryCommand::SetState {
        chapter_id: c.chapter.id,
        state: WorkspaceState::Active,
    });
    assert_eq!(restored.links[0].id, link_id);
    f.apply(StoryCommand::Unlink {
        chapter_id: c.chapter.id,
        link_id,
    });
    assert!(f.worker.story_usage(thron).unwrap().is_empty());
}
#[test]
fn reorder_is_atomic_and_preserves_document_and_link_identity() {
    let f = Fixture::new();
    let a = f.chapter("A");
    let b = f.chapter("B");
    let c = f.chapter("C");
    f.apply(StoryCommand::Move {
        chapter_id: c.chapter.id,
        before_id: Some(a.chapter.id),
    });
    assert_eq!(
        f.worker
            .read_story()
            .unwrap()
            .chapters
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        vec![c.chapter.id, a.chapter.id, b.chapter.id]
    );
    f.db().execute_batch("CREATE TRIGGER fail_order BEFORE UPDATE OF reading_rank ON story_unit WHEN NEW.reading_rank > 0 BEGIN SELECT RAISE(ABORT,'injected order failure'); END;").unwrap();
    let rev = f.worker.read_story().unwrap().global_revision;
    assert!(f
        .worker
        .apply_story(
            rev,
            StoryCommand::Move {
                chapter_id: a.chapter.id,
                before_id: None
            }
        )
        .is_err());
    let index = f.worker.read_story().unwrap();
    assert_eq!(index.global_revision, rev);
    assert!(index.chapters.iter().all(|c| c.reading_rank > 0));
    assert_eq!(index.chapters[0].id, c.chapter.id);
    assert_eq!(
        f.worker.read_chapter(a.chapter.id).unwrap().documents[0].id,
        a.documents[0].id
    );
}
#[test]
fn stale_and_concurrent_writes_cannot_overwrite_each_other() {
    let f = Fixture::new();
    let c = f.chapter("A");
    let rev = c.global_revision;
    let results = std::thread::scope(|scope| {
        let worker = &f.worker;
        let id = c.chapter.id;
        let a = scope.spawn(move || {
            worker.apply_story(
                rev,
                StoryCommand::Save {
                    chapter_id: id,
                    title: Some("First".into()),
                    documents: vec![],
                },
            )
        });
        let b = scope.spawn(move || {
            worker.apply_story(
                rev,
                StoryCommand::Save {
                    chapter_id: id,
                    title: Some("Second".into()),
                    documents: vec![],
                },
            )
        });
        [a.join().unwrap().is_ok(), b.join().unwrap().is_ok()]
    });
    assert_eq!(results.iter().filter(|v| **v).count(), 1);
    assert!(f
        .worker
        .apply_story(
            rev,
            StoryCommand::Save {
                chapter_id: c.chapter.id,
                title: None,
                documents: vec![edit(DocumentArea::Manuscript, "stale")]
            }
        )
        .is_err());
    assert!(f
        .worker
        .read_chapter(c.chapter.id)
        .unwrap()
        .documents
        .iter()
        .all(|d| d.plain_text.trim().is_empty()));
}
#[test]
fn newer_or_corrupt_documents_are_preserved_and_block_only_their_own_area() {
    for newer in [true, false] {
        let f = Fixture::new();
        let c = f.chapter("Recovery");
        let original = if newer {
            r#"{"type":"doc","content":[{"type":"paragraph"}]}"#
        } else {
            "{broken"
        };
        f.db().execute("UPDATE rich_document SET canonical_json=?1,document_schema_version=?2,plain_text='Preserved text' WHERE owner_id=?3 AND area='manuscript'",params![original,if newer{2}else{1},c.chapter.id.to_string()]).unwrap();
        let read = f.worker.read_chapter(c.chapter.id).unwrap();
        let doc = read
            .documents
            .iter()
            .find(|d| d.area == DocumentArea::Manuscript)
            .unwrap();
        assert!(doc.read_only_reason.is_some());
        assert!(doc.content.is_none());
        assert_eq!(doc.original_json.as_deref(), Some(original));
        assert_eq!(doc.plain_text, "Preserved text");
        assert!(f
            .worker
            .apply_story(
                read.global_revision,
                StoryCommand::Save {
                    chapter_id: c.chapter.id,
                    title: Some("Must roll back".into()),
                    documents: vec![edit(DocumentArea::Manuscript, "Overwrite")]
                }
            )
            .is_err());
        f.apply(StoryCommand::Save {
            chapter_id: c.chapter.id,
            title: None,
            documents: vec![edit(DocumentArea::Notes, "Still editable")],
        });
        let read = f.worker.read_chapter(c.chapter.id).unwrap();
        assert_eq!(read.chapter.title, "Recovery");
        assert!(read
            .documents
            .iter()
            .any(|d| d.original_json.as_deref() == Some(original)));
        assert!(read
            .documents
            .iter()
            .any(|d| d.plain_text.contains("Still editable")));
    }
}
#[test]
fn document_validation_rejects_lossy_or_unsafe_content_and_counts_unicode_words() {
    let rich = json!({"type":"doc","content":[{"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"Über die Welt","marks":[{"type":"bold"}]}]},{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Привет мир"}]}]}]}]});
    let (plain, count) = document_text(1, &rich).unwrap();
    assert!(plain.contains("Welt\nПривет"));
    assert_eq!(count, 5);
    for value in [
        json!({"type":"doc","content":[{"type":"script","text":"alert(1)"}]}),
        json!({"type":"doc","content":[{"type":"text","text":"bad parent"}]}),
        json!({"type":"doc","content":[{"type":"paragraph","attrs":{"onclick":"bad"}}]}),
        json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"hello","marks":[{"type":"unknown"}]}]}]}),
        json!({"type":"doc","content":[]}),
    ] {
        assert!(document_text(1, &value).is_err());
    }
    assert!(document_text(2, &rich).is_err());
    let mut deep = json!({"type":"paragraph"});
    for _ in 0..66 {
        deep = json!({"type":"blockquote","content":[deep]});
    }
    assert!(document_text(1, &json!({"type":"doc","content":[deep]})).is_err());
}
#[test]
fn bad_roles_targets_or_documents_roll_back_the_complete_command() {
    let f = Fixture::new();
    let c = f.chapter("A");
    let entry = f.entry("Thron");
    let linked = f.apply(StoryCommand::SetLink {
        chapter_id: c.chapter.id,
        entry_id: entry,
        role_ids: vec![c.roles[0].id.clone()],
    });
    let rev = linked.global_revision;
    assert!(f
        .worker
        .apply_story(
            rev,
            StoryCommand::SetLink {
                chapter_id: c.chapter.id,
                entry_id: entry,
                role_ids: vec!["not-here".into()]
            }
        )
        .is_err());
    assert_eq!(
        f.worker.read_chapter(c.chapter.id).unwrap().links[0]
            .roles
            .len(),
        1
    );
    assert!(f
        .worker
        .apply_story(
            rev,
            StoryCommand::SetLink {
                chapter_id: c.chapter.id,
                entry_id: EntryId::new(),
                role_ids: vec![]
            }
        )
        .is_err());
    let mut bad = edit(DocumentArea::Notes, "Bad");
    bad.content = json!({"type":"unknown"});
    assert!(f
        .worker
        .apply_story(
            rev,
            StoryCommand::Save {
                chapter_id: c.chapter.id,
                title: Some("bad".into()),
                documents: vec![edit(DocumentArea::Manuscript, "Must roll back"), bad]
            }
        )
        .is_err());
    let next = f.worker.read_chapter(c.chapter.id).unwrap();
    assert_eq!(next.global_revision, rev);
    assert!(next
        .documents
        .iter()
        .all(|d| d.plain_text.trim().is_empty()));
    assert!(f
        .db()
        .execute(
            "INSERT INTO story_link VALUES('wrong',?1,?1,NULL,'now','now',1)",
            [c.chapter.id.to_string()]
        )
        .is_err());
    assert!(f
        .db()
        .execute(
            "DELETE FROM record_identity WHERE record_id=?1",
            [entry.to_string()]
        )
        .is_err());
}
#[test]
fn acknowledged_manuscript_survives_abrupt_exit_and_unfinished_write_rolls_back() {
    let f = Fixture::new();
    let c = f.chapter("Abrupt exit");
    let path = f.dir.path().join("project.sqlite");
    f.worker.shutdown().unwrap();
    for mode in ["committed", "unfinished"] {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "story_process_exit_helper", "--ignored"])
            .env("WC_STORY_TEST_DB", &path)
            .env("WC_STORY_TEST_PROJECT", f.project.to_string())
            .env("WC_STORY_TEST_CHAPTER", c.chapter.id.to_string())
            .env("WC_STORY_TEST_MODE", mode)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        let worker = ProjectDbWorker::spawn(path.clone(), f.project, None).unwrap();
        let read = worker.read_chapter(c.chapter.id).unwrap();
        assert!(read
            .documents
            .iter()
            .any(|d| d.plain_text.contains("Committed writing")));
        worker.shutdown().unwrap();
    }
}
#[test]
#[ignore = "subprocess helper invoked by the abrupt-exit integration test"]
fn story_process_exit_helper() {
    let path = std::path::PathBuf::from(std::env::var("WC_STORY_TEST_DB").unwrap());
    let chapter_id = ChapterId::parse(&std::env::var("WC_STORY_TEST_CHAPTER").unwrap()).unwrap();
    if std::env::var("WC_STORY_TEST_MODE").unwrap() == "committed" {
        let worker = ProjectDbWorker::spawn(
            path,
            ProjectId::parse(&std::env::var("WC_STORY_TEST_PROJECT").unwrap()).unwrap(),
            None,
        )
        .unwrap();
        worker
            .apply_story(
                worker.read_story().unwrap().global_revision,
                StoryCommand::Save {
                    chapter_id,
                    title: None,
                    documents: vec![edit(DocumentArea::Manuscript, "Committed writing")],
                },
            )
            .unwrap();
    } else {
        let db = Connection::open(path).unwrap();
        db.execute_batch(
            "BEGIN IMMEDIATE; UPDATE rich_document SET plain_text='Unfinished overwrite'",
        )
        .unwrap();
        std::process::exit(0);
    }
    std::process::exit(0);
}

#[test]
fn schema_eight_upgrade_is_atomic_and_keeps_a_recovery_snapshot() {
    use worldcrafter_lib::{
        application::{AppState, ProjectService},
        package::manifest::Manifest,
    };
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let project = ProjectService::create_project(&state, dir.path(), "Before Chapters").unwrap();
    let entry = ProjectService::create_entry(
        &state,
        project.project_id,
        None,
        None,
        Some("Old Entry".into()),
    )
    .unwrap();
    ProjectService::close_project(&state, project.project_id).unwrap();
    let root = std::path::Path::new(&project.package_path);
    let db = Connection::open(root.join("data/project.sqlite")).unwrap();
    db.execute_batch("DROP TABLE project_pin; DROP TABLE project_recent; DROP TABLE project_navigation_settings; DROP TRIGGER rich_document_owner_insert; DROP TRIGGER rich_document_owner_update; DROP TRIGGER rich_document_keep_entry; DROP TRIGGER rich_document_keep_chapter; DROP TABLE occurrence_entry; DROP TABLE occurrence_chapter; DROP TABLE temporal_occurrence; DROP TRIGGER occurrence_event_preserve; DROP TABLE timeline_calendar; DELETE FROM capability_def WHERE id='event'; DROP TRIGGER search_source_updated; DROP TRIGGER search_source_created; DROP TABLE search_index; DROP TABLE derived_index_state; DROP TABLE entry_alias; DROP TABLE story_link_role; DROP TABLE story_link; DROP TABLE story_role; DROP TABLE rich_document; DROP TABLE story_unit; PRAGMA user_version=8; UPDATE project_meta SET schema_version=8; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;").unwrap();
    let mut manifest = Manifest::read(&root.join("manifest.json")).unwrap();
    manifest.schema_version = 8;
    manifest.write(&root.join("manifest.json")).unwrap();
    assert!(ProjectService::open_project(&state, root, false).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        8
    );
    assert!(db.prepare("SELECT * FROM story_unit").is_err());
    db.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(db);
    let open = ProjectService::open_project(&state, root, false).unwrap();
    assert_eq!(
        open.schema_version,
        worldcrafter_lib::persistence::migrations::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(
        ProjectService::get_entry(&state, open.project_id, entry.id)
            .unwrap()
            .authored_name
            .as_deref(),
        Some("Old Entry")
    );
    assert!(ProjectService::read_story(&state, open.project_id)
        .unwrap()
        .chapters
        .is_empty());
    let recovery = Connection::open_with_flags(
        dir.path()
            .join(".worldcrafter-migration-recovery")
            .join(open.project_id.to_string())
            .join("schema-v8.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        8
    );
    assert_eq!(
        recovery
            .query_row("SELECT authored_name FROM entry", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "Old Entry"
    );
    ProjectService::close_project(&state, open.project_id).unwrap();
}

#[test]
fn backup_restore_preserves_chapter_documents_and_internal_link_ids() {
    use worldcrafter_lib::application::{AppState, ProjectService};
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let source = ProjectService::create_project(&state, dir.path(), "Story source").unwrap();
    let entry = ProjectService::create_entry(
        &state,
        source.project_id,
        None,
        None,
        Some("Traveller".into()),
    )
    .unwrap();
    let revision = ProjectService::read_story(&state, source.project_id)
        .unwrap()
        .global_revision;
    let c = ProjectService::apply_story(
        &state,
        source.project_id,
        revision,
        StoryCommand::Create {
            title: "A beginning".into(),
        },
    )
    .unwrap();
    let c = ProjectService::apply_story(
        &state,
        source.project_id,
        c.global_revision,
        StoryCommand::Save {
            chapter_id: c.chapter.id,
            title: None,
            documents: vec![
                edit(DocumentArea::Manuscript, "An unchanged manuscript."),
                edit(DocumentArea::Notes, "Notes are separate."),
            ],
        },
    )
    .unwrap();
    let c = ProjectService::apply_story(
        &state,
        source.project_id,
        c.global_revision,
        StoryCommand::SetLink {
            chapter_id: c.chapter.id,
            entry_id: entry.id,
            role_ids: vec![c.roles[0].id.clone(), c.roles[1].id.clone()],
        },
    )
    .unwrap();
    let backup =
        ProjectService::create_backup(&state, source.project_id, &dir.path().join("backups"))
            .unwrap();
    let copy =
        ProjectService::restore_backup_as_copy(&state, &backup, dir.path(), Some("Story copy"))
            .unwrap();
    assert_ne!(source.project_id, copy.project_id);
    let restored = ProjectService::read_chapter(&state, copy.project_id, c.chapter.id).unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&c).unwrap()
    );
    let usage = ProjectService::story_usage(&state, copy.project_id, entry.id).unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].roles.len(), 2);
    let modified = ProjectService::apply_story(
        &state,
        copy.project_id,
        restored.global_revision,
        StoryCommand::Save {
            chapter_id: c.chapter.id,
            title: Some("Independent title".into()),
            documents: vec![],
        },
    )
    .unwrap();
    assert_ne!(modified.chapter.title, c.chapter.title);
    assert_eq!(
        ProjectService::read_chapter(&state, source.project_id, c.chapter.id)
            .unwrap()
            .chapter
            .title,
        c.chapter.title
    );
    ProjectService::close_project(&state, source.project_id).unwrap();
    ProjectService::close_project(&state, copy.project_id).unwrap();
}
