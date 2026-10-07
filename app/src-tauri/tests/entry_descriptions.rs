use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::Path;
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        entry_description::EntryDescriptionSnapshot,
        lifecycle::StructureCommand,
        search::{SearchRequest, SearchTarget},
        story::{DocumentArea, DocumentEdit, StoryCommand, WorkspaceState},
        Entry, EntryId, ProjectId,
    },
    package::Manifest,
    persistence::migrations::CURRENT_SCHEMA_VERSION,
};

struct Fixture {
    state: AppState,
    dir: TempDir,
    project: ProjectId,
    path: String,
    entry: Entry,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let project =
            ProjectService::create_project(&state, dir.path(), "Description fixture").unwrap();
        let entry =
            ProjectService::create_entry(&state, project.project_id, None, None, None).unwrap();
        Self {
            state,
            dir,
            project: project.project_id,
            path: project.package_path,
            entry,
        }
    }
    fn read(&self) -> EntryDescriptionSnapshot {
        ProjectService::read_entry_description(&self.state, self.project, self.entry.id).unwrap()
    }
    fn save(&self, content: Value) -> EntryDescriptionSnapshot {
        ProjectService::save_entry_description(
            &self.state,
            self.project,
            self.entry.id,
            self.read().global_revision,
            self.read().document.as_ref().map(|d| d.revision),
            1,
            content,
        )
        .unwrap()
    }
    fn db(&self) -> Connection {
        let db = Connection::open(Path::new(&self.path).join("data/project.sqlite")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db
    }
    fn search(
        &self,
        query: &str,
        include_inactive: bool,
    ) -> Vec<worldcrafter_lib::domain::search::SearchHit> {
        ProjectService::search_project(
            &self.state,
            self.project,
            SearchRequest {
                query: query.into(),
                include_inactive,
                limit_per_group: 20,
                entry_id: Some(self.entry.id),
                structured_kind: None,
                text_area: None,
                story_role_id: None,
            },
        )
        .unwrap()
        .groups
        .into_iter()
        .find(|g| g.kind == "text")
        .unwrap()
        .hits
    }
    fn set_state(&self, state: WorkspaceState) {
        ProjectService::apply_structure(
            &self.state,
            self.project,
            self.read().global_revision,
            StructureCommand::SetEntryState {
                id: self.entry.id,
                state,
            },
            None,
        )
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}
fn prose(text: &str) -> Value {
    json!({"type":"doc","content":[{"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"Biography"}]},{"type":"paragraph","content":[{"type":"text","text":text,"marks":[{"type":"bold"}]}]},{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"A detail"}]}]}]}]})
}

#[test]
fn optional_description_round_trips_independently_from_entry_metadata() {
    let f = Fixture::new();
    assert!(f.read().document.is_none());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM rich_document WHERE owner_kind='entry'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    let content = prose("Invented explorer — Ära 雪");
    let first = f.save(content.clone());
    assert_eq!(
        first.document.as_ref().unwrap().content,
        Some(content.clone())
    );
    assert!(first
        .document
        .as_ref()
        .unwrap()
        .plain_text
        .contains("Ära 雪"));
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, f.entry.id)
            .unwrap()
            .revision,
        f.entry.revision
    );
    let second = f.save(prose("Revised explorer"));
    assert_eq!(
        first.document.as_ref().unwrap().id,
        second.document.as_ref().unwrap().id
    );
    assert_eq!(
        second.document.as_ref().unwrap().revision,
        first.document.as_ref().unwrap().revision + 1
    );
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert_eq!(
        serde_json::to_value(f.read()).unwrap(),
        serde_json::to_value(second).unwrap()
    );
    let empty = f.save(json!({"type":"doc","content":[{"type":"paragraph"}]}));
    assert_eq!(empty.document.as_ref().unwrap().word_count, 0);
    assert_eq!(empty.document.unwrap().id, first.document.unwrap().id);
}

#[test]
fn description_survives_rename_reclassification_lifecycle_and_restored_copy() {
    let f = Fixture::new();
    let description = f
        .save(prose("An expedition to the invented citadel"))
        .document
        .unwrap();
    let renamed = ProjectService::update_entry_name(
        &f.state,
        f.project,
        f.entry.id,
        f.entry.revision,
        Some("New label".into()),
    )
    .unwrap();
    let category = ProjectService::create_category(&f.state, f.project, "Explorers").unwrap();
    let kind =
        ProjectService::create_type(&f.state, f.project, category.id, None, "Navigator").unwrap();
    ProjectService::change_entry_structure(
        &f.state,
        f.project,
        f.entry.id,
        renamed.revision,
        category.id,
        Some(kind.id),
    )
    .unwrap();
    for state in [WorkspaceState::Archived, WorkspaceState::Trashed] {
        f.set_state(state);
        assert_eq!(
            f.read().document.as_ref().unwrap().content,
            description.content
        );
        assert!(ProjectService::save_entry_description(
            &f.state,
            f.project,
            f.entry.id,
            f.read().global_revision,
            f.read().document.as_ref().map(|d| d.revision),
            1,
            prose("Blocked write")
        )
        .is_err());
        assert!(f.search("citadel", false).is_empty());
        assert_eq!(f.search("citadel", true).len(), 1);
        f.set_state(WorkspaceState::Active);
    }
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let restored = ProjectService::restore_backup_as_copy(
        &f.state,
        &backup,
        &f.dir.path().join("restored"),
        Some("Copy"),
    )
    .unwrap();
    assert_ne!(restored.project_id, f.project);
    let copied = ProjectService::read_entry_description(&f.state, restored.project_id, f.entry.id)
        .unwrap()
        .document
        .unwrap();
    assert_eq!(
        serde_json::to_value(copied).unwrap(),
        serde_json::to_value(description).unwrap()
    );
    ProjectService::close_project(&f.state, restored.project_id).unwrap();
}

#[test]
fn stale_wrong_project_missing_entry_and_unsupported_writes_preserve_saved_content() {
    let f = Fixture::new();
    let stale = f.read().global_revision;
    let before = f.save(prose("Keep this writing"));
    let error = ProjectService::save_entry_description(
        &f.state,
        f.project,
        f.entry.id,
        stale,
        f.read().document.as_ref().map(|d| d.revision),
        1,
        prose("Stale"),
    )
    .unwrap_err();
    assert_eq!(error.kind(), "revision_conflict");
    assert!(ProjectService::save_entry_description(
        &f.state,
        ProjectId::new(),
        f.entry.id,
        before.global_revision,
        f.read().document.as_ref().map(|d| d.revision),
        1,
        prose("Wrong Project")
    )
    .is_err());
    assert!(ProjectService::save_entry_description(
        &f.state,
        f.project,
        EntryId::new(),
        before.global_revision,
        f.read().document.as_ref().map(|d| d.revision),
        1,
        prose("Missing Entry")
    )
    .is_err());
    let other = ProjectService::create_project(&f.state, f.dir.path(), "Other Project").unwrap();
    assert!(
        ProjectService::read_entry_description(&f.state, other.project_id, f.entry.id).is_err()
    );
    assert!(ProjectService::save_entry_description(
        &f.state,
        other.project_id,
        f.entry.id,
        other.revision,
        f.read().document.as_ref().map(|d| d.revision),
        1,
        prose("Foreign Entry")
    )
    .is_err());
    ProjectService::close_project(&f.state, other.project_id).unwrap();
    for (version, content) in [
        (99, prose("Future")),
        (
            1,
            json!({"type":"doc","content":[{"type":"image","attrs":{"src":"file:///invalid"}}]}),
        ),
    ] {
        assert!(ProjectService::save_entry_description(
            &f.state,
            f.project,
            f.entry.id,
            before.global_revision,
            f.read().document.as_ref().map(|d| d.revision),
            version,
            content
        )
        .is_err());
    }
    assert_eq!(
        serde_json::to_value(f.read()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
}

#[test]
fn damaged_future_and_recovery_documents_remain_read_only_with_original_bytes() {
    let f = Fixture::new();
    f.save(prose("Preserved searchable fallback"));
    for (version, raw, migration) in [
        (99, prose("Future prose").to_string(), "current"),
        (1, "{ broken authored bytes".into(), "current"),
        (1, json!({"type":"unsupported"}).to_string(), "current"),
        (1, prose("Recovery text").to_string(), "pending"),
    ] {
        f.db().execute("UPDATE rich_document SET document_schema_version=?1,canonical_json=?2,migration_state=?3 WHERE owner_id=?4",params![version,raw,migration,f.entry.id.to_string()]).unwrap();
        let snapshot = f.read();
        let document = snapshot.document.unwrap();
        assert!(document.content.is_none());
        assert!(document.read_only_reason.is_some());
        assert_eq!(document.original_json.as_deref(), Some(raw.as_str()));
        assert!(ProjectService::save_entry_description(
            &f.state,
            f.project,
            f.entry.id,
            snapshot.global_revision,
            f.read().document.as_ref().map(|d| d.revision),
            1,
            prose("Overwrite attempt")
        )
        .is_err());
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT canonical_json FROM rich_document WHERE owner_id=?1",
                    [f.entry.id.to_string()],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            raw
        );
    }
}

#[test]
fn descriptions_are_searchable_refresh_after_edits_and_never_create_links() {
    let f = Fixture::new();
    f.save(prose("Invented observatory aurora"));
    for _ in 0..2 {
        let results = f.search("observatory", false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].context, "Description · Text match");
        assert_eq!(
            results[0].target,
            SearchTarget::Entry {
                entry_id: f.entry.id.to_string()
            }
        );
        assert!(results[0].excerpt.contains("observatory"));
    }
    f.save(prose("Invented lighthouse"));
    assert!(f.search("observatory", false).is_empty());
    assert_eq!(f.search("lighthouse", false).len(), 1);
    f.db().execute_batch("DROP TABLE search_index").unwrap();
    assert_eq!(f.search("lighthouse", false).len(), 1);
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM relationship_instance", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM story_link", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn injected_save_failure_rolls_back_document_and_global_revision() {
    let f = Fixture::new();
    let before = f.save(prose("Original"));
    f.db().execute_batch("CREATE TRIGGER fail_description_save BEFORE UPDATE OF last_committed_revision ON project_meta BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(ProjectService::save_entry_description(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        f.read().document.as_ref().map(|d| d.revision),
        1,
        prose("Interrupted")
    )
    .is_err());
    assert_eq!(
        serde_json::to_value(f.read()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    f.db()
        .execute_batch("DROP TRIGGER fail_description_save")
        .unwrap();
    assert!(f
        .save(prose("Retry"))
        .document
        .unwrap()
        .plain_text
        .contains("Retry"));
}

#[test]
fn existing_description_field_keeps_its_identity_and_value() {
    use worldcrafter_lib::domain::fields::*;
    let f = Fixture::new();
    let fields = ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        f.read().global_revision,
        FieldCommand::Create {
            name: "Description".into(),
            unit: None,
            field_kind: FieldKind::ShortText,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: f.entry.id.to_string(),
            },
            options: vec![],
            value: Some(FieldValue::Text(
                "Keep the existing structured value".into(),
            )),
        },
    )
    .unwrap();
    f.save(prose("Independent rich prose"));
    let after = ProjectService::read_fields(&f.state, f.project, f.entry.id).unwrap();
    assert_eq!(
        serde_json::to_value(fields.fields).unwrap(),
        serde_json::to_value(after.fields).unwrap()
    );
}

#[test]
fn concurrent_description_updates_serialize_and_reject_one_stale_writer() {
    let f = Fixture::new();
    let revision = f.read().global_revision;
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let threads = ["First draft", "Second draft"].map(|text| {
            let state = &f.state;
            let barrier = &barrier;
            let project = f.project;
            let entry = f.entry.id;
            scope.spawn(move || {
                barrier.wait();
                ProjectService::save_entry_description(
                    state,
                    project,
                    entry,
                    revision,
                    None,
                    1,
                    prose(text),
                )
            })
        });
        threads.map(|t| t.join().unwrap())
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .find_map(|r| r.as_ref().err())
            .unwrap()
            .kind(),
        "revision_conflict"
    );
    let accepted = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(
        serde_json::to_value(f.read()).unwrap(),
        serde_json::to_value(accepted).unwrap()
    );
}

#[test]
fn refreshing_the_project_revision_cannot_hide_a_conflicting_description_edit() {
    let f = Fixture::new();
    let absent = f.read();
    let first = f.save(prose("First writer"));
    let second = f.save(prose("Another writer's newer description"));
    // A caller may learn a fresh global revision from an unrelated Field read.
    // It must still compare the exact description revision it originally saw.
    for expected_document in [
        absent.document.map(|d| d.revision),
        first.document.map(|d| d.revision),
    ] {
        let error = ProjectService::save_entry_description(
            &f.state,
            f.project,
            f.entry.id,
            second.global_revision,
            expected_document,
            1,
            prose("Old draft with refreshed global revision"),
        )
        .unwrap_err();
        assert_eq!(error.kind(), "revision_conflict");
        assert_eq!(
            serde_json::to_value(f.read()).unwrap(),
            serde_json::to_value(&second).unwrap()
        );
    }
}

#[test]
fn chapter_area_search_filters_do_not_mislabel_entry_descriptions() {
    let f = Fixture::new();
    f.save(prose("Shared phrase"));
    let c = ProjectService::apply_story(
        &f.state,
        f.project,
        f.read().global_revision,
        StoryCommand::Create {
            title: "Chapter".into(),
        },
    )
    .unwrap();
    ProjectService::apply_story(
        &f.state,
        f.project,
        c.global_revision,
        StoryCommand::Save {
            chapter_id: c.chapter.id,
            title: None,
            documents: vec![DocumentEdit {
                area: DocumentArea::Plan,
                schema_version: 1,
                content: prose("Shared phrase"),
            }],
        },
    )
    .unwrap();
    let result = ProjectService::search_project(
        &f.state,
        f.project,
        SearchRequest {
            query: "Shared phrase".into(),
            include_inactive: false,
            limit_per_group: 20,
            entry_id: None,
            structured_kind: None,
            text_area: Some(DocumentArea::Plan),
            story_role_id: None,
        },
    )
    .unwrap();
    let hits = &result
        .groups
        .iter()
        .find(|g| g.kind == "text")
        .unwrap()
        .hits;
    assert_eq!(hits.len(), 1);
    assert_eq!(
        hits[0].target,
        SearchTarget::Chapter {
            chapter_id: c.chapter.id.to_string(),
            area: "plan".into()
        }
    );
}

#[test]
fn description_ownership_and_document_area_are_enforced_by_database() {
    let f = Fixture::new();
    let doc = f.save(prose("Owned writing")).document.unwrap();
    let db = f.db();
    assert!(db
        .execute(
            "UPDATE rich_document SET area='manuscript' WHERE id=?1",
            [&doc.id]
        )
        .is_err());
    assert!(db
        .execute(
            "UPDATE rich_document SET owner_id=?1 WHERE id=?2",
            params![EntryId::new().to_string(), doc.id]
        )
        .is_err());
    assert!(db
        .execute("DELETE FROM entry WHERE id=?1", [f.entry.id.to_string()])
        .is_err());
    assert_eq!(f.read().document.unwrap().id, doc.id);
}

#[test]
fn schema_eleven_migration_preserves_chapters_and_rolls_back_atomically() {
    let f = Fixture::new();
    let c = ProjectService::apply_story(
        &f.state,
        f.project,
        f.read().global_revision,
        StoryCommand::Create {
            title: "Existing Chapter".into(),
        },
    )
    .unwrap();
    let c = ProjectService::apply_story(
        &f.state,
        f.project,
        c.global_revision,
        StoryCommand::Save {
            chapter_id: c.chapter.id,
            title: None,
            documents: vec![DocumentEdit {
                area: DocumentArea::Manuscript,
                schema_version: 1,
                content: prose("Older chapter prose"),
            }],
        },
    )
    .unwrap();
    // A newer Chapter document must survive the table migration byte for byte,
    // even though this build cannot edit that document schema.
    f.db().execute("UPDATE rich_document SET document_schema_version=99,canonical_json=?1 WHERE owner_id=?2 AND area='notes'",params!["{\"future\": \"keep this spacing\"}",c.chapter.id.to_string()]).unwrap();
    let c = ProjectService::read_chapter(&f.state, f.project, c.chapter.id).unwrap();
    ProjectService::close_project(&f.state, f.project).unwrap();
    let db = f.db();
    db.execute_batch("DROP TRIGGER rich_document_owner_insert; DROP TRIGGER rich_document_owner_update; DROP TRIGGER rich_document_keep_entry; DROP TRIGGER rich_document_keep_chapter; ALTER TABLE rich_document RENAME TO rich_document_new;").unwrap();
    let old = include_str!("../src/persistence/migrations/0009_story.sql");
    let start = old.find("CREATE TABLE rich_document (").unwrap();
    let end = old.find("CREATE TABLE story_role (").unwrap();
    db.execute_batch(&old[start..end]).unwrap();
    db.execute_batch("INSERT INTO rich_document SELECT * FROM rich_document_new; DROP TABLE rich_document_new; PRAGMA user_version=11; UPDATE project_meta SET schema_version=11; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;").unwrap();
    let root = Path::new(&f.path);
    let mut manifest = Manifest::read(&root.join("manifest.json")).unwrap();
    manifest.schema_version = 11;
    manifest.write(&root.join("manifest.json")).unwrap();
    assert!(ProjectService::open_project(&f.state, root, false).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        11
    );
    assert!(!db
        .prepare("SELECT 1 FROM sqlite_master WHERE name='rich_document_keep_entry'")
        .unwrap()
        .exists([])
        .unwrap());
    assert_eq!(
        db.query_row(
            "SELECT canonical_json FROM rich_document WHERE owner_id=?1 AND area='manuscript'",
            [c.chapter.id.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        prose("Older chapter prose").to_string()
    );
    db.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(db);
    let opened = ProjectService::open_project(&f.state, root, false).unwrap();
    assert_eq!(opened.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(
        serde_json::to_value(
            ProjectService::read_chapter(&f.state, f.project, c.chapter.id).unwrap()
        )
        .unwrap(),
        serde_json::to_value(c).unwrap()
    );
    assert!(f.read().document.is_none());
    f.save(prose("New description after migration"));
    assert!(!f
        .db()
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    let recovery = Connection::open_with_flags(
        f.dir
            .path()
            .join(".worldcrafter-migration-recovery")
            .join(f.project.to_string())
            .join("schema-v11.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        11
    );
    assert_eq!(
        recovery
            .query_row(
                "SELECT count(*) FROM rich_document WHERE owner_kind='entry'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
