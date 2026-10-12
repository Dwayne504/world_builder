use rusqlite::{params, Connection};
use serde_json::json;
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{fields::*, relationships::*, search::*, story::*, Entry, EntryId, ProjectId},
    package::Manifest,
    persistence::migrations::CURRENT_SCHEMA_VERSION,
};

struct Fixture {
    dir: TempDir,
    state: AppState,
    project: ProjectId,
    path: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let p = ProjectService::create_project(&state, dir.path(), "Search fixture").unwrap();
        Self {
            dir,
            state,
            project: p.project_id,
            path: p.package_path,
        }
    }
    fn entry(&self, name: &str) -> Entry {
        ProjectService::create_entry(&self.state, self.project, None, None, Some(name.into()))
            .unwrap()
    }
    fn revision(&self) -> i64 {
        self.db()
            .query_row(
                "SELECT last_committed_revision FROM project_meta",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }
    fn db(&self) -> Connection {
        let db =
            Connection::open(std::path::Path::new(&self.path).join("data/project.sqlite")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db
    }
    fn alias(&self, entry: EntryId, text: &str) -> EntryAliases {
        ProjectService::apply_alias(
            &self.state,
            self.project,
            entry,
            self.revision(),
            AliasCommand::Add { text: text.into() },
        )
        .unwrap()
    }
    fn search(&self, query: &str) -> SearchResults {
        ProjectService::search_project(&self.state, self.project, request(query)).unwrap()
    }
    fn story(&self, command: StoryCommand) -> ChapterSnapshot {
        ProjectService::apply_story(&self.state, self.project, self.revision(), command).unwrap()
    }
    fn chapter(&self, title: &str, text: &str) -> ChapterSnapshot {
        let c = self.story(StoryCommand::Create {
            title: title.into(),
        });
        self.story(StoryCommand::Save{chapter_id:c.chapter.id,title:None,documents:vec![DocumentEdit{area:DocumentArea::Manuscript,schema_version:1,content:json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})}]})
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}
fn request(query: &str) -> SearchRequest {
    SearchRequest {
        query: query.into(),
        include_inactive: false,
        limit_per_group: 10,
        entry_id: None,
        structured_kind: None,
        text_area: None,
        story_role_id: None,
    }
}
fn hits(result: &SearchResults, group: &str) -> Vec<SearchHit> {
    result
        .groups
        .iter()
        .find(|g| g.kind == group)
        .unwrap()
        .hits
        .clone()
}

#[test]
fn exact_names_then_aliases_then_prefixes_stay_above_prose() {
    let f = Fixture::new();
    let exact = f.entry("Luna");
    let alias = f.entry("Night Queen");
    f.alias(alias.id, "Luna");
    f.entry("Lunar Guard");
    let c = f.chapter("Luna", "Luna Luna Luna is a word in prose.");
    let results = f.search("luna");
    let entries = hits(&results, "entries");
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].title, "Luna");
    assert_eq!(entries[1].title, "Night Queen");
    assert_eq!(entries[1].reason, "Alias: Luna");
    assert_eq!(entries[2].title, "Lunar Guard");
    assert_eq!(
        hits(&results, "chapters")[0].target,
        SearchTarget::Chapter {
            chapter_id: c.chapter.id.to_string(),
            area: "manuscript".into()
        }
    );
    assert!(hits(&results, "text")[0]
        .reason
        .contains("not a structural link"));
    assert_eq!(
        hits(&f.search(&exact.id.to_string()), "entries")[0].target,
        SearchTarget::Entry {
            entry_id: exact.id.to_string()
        }
    );
    assert_eq!(
        serde_json::to_value(f.search("luna")).unwrap(),
        serde_json::to_value(results).unwrap()
    );
}
#[test]
fn aliases_are_explicit_stable_scoped_and_do_not_rewrite_prose_on_rename() {
    let f = Fixture::new();
    let a = f.entry("Old name");
    let b = f.entry("Other");
    let aliases = f.alias(a.id, "The Wanderer");
    f.alias(b.id, "The Wanderer");
    let c = f.chapter("Journey", "Old name called The Wanderer.");
    assert_eq!(hits(&f.search("the wanderer"), "entries").len(), 2);
    let rev = f.revision();
    assert!(ProjectService::apply_alias(
        &f.state,
        f.project,
        a.id,
        rev,
        AliasCommand::Add {
            text: " THE   WANDERER ".into()
        }
    )
    .is_err());
    assert_eq!(f.revision(), rev);
    ProjectService::update_entry_name(
        &f.state,
        f.project,
        a.id,
        a.revision,
        Some("New name".into()),
    )
    .unwrap();
    let after = ProjectService::read_aliases(&f.state, f.project, a.id).unwrap();
    assert_eq!(aliases.aliases, after.aliases);
    assert!(hits(&f.search("Old name"), "entries").is_empty());
    assert_eq!(hits(&f.search("New name"), "entries")[0].title, "New name");
    assert_eq!(
        ProjectService::read_chapter(&f.state, f.project, c.chapter.id)
            .unwrap()
            .documents
            .iter()
            .find(|d| d.area == DocumentArea::Manuscript)
            .unwrap()
            .plain_text,
        "Old name called The Wanderer.\n"
    );
    assert!(ProjectService::apply_alias(
        &f.state,
        f.project,
        b.id,
        f.revision(),
        AliasCommand::Delete {
            alias_id: aliases.aliases[0].id.clone()
        }
    )
    .is_err());
    ProjectService::apply_alias(
        &f.state,
        f.project,
        a.id,
        f.revision(),
        AliasCommand::Delete {
            alias_id: aliases.aliases[0].id.clone(),
        },
    )
    .unwrap();
    assert_eq!(hits(&f.search("wanderer"), "entries").len(), 1);
}
#[test]
fn unicode_case_accents_and_literal_query_syntax_agree_in_index_and_fallback() {
    let f = Fixture::new();
    let e = f.entry("Élan d’Æther");
    f.alias(e.id, "北方 王");
    f.alias(e.id, "Starship 👑");
    for query in [
        "👑",
        "ELAN",
        "ÉLAN",
        "北方",
        "Starship",
        "Æth",
        "d’Æ",
        "\"Élan\"",
        "*",
        "' OR 1=1 --",
        "() NEAR AND",
    ] {
        f.db()
            .execute("UPDATE derived_index_state SET dirty=1", [])
            .unwrap();
        let a = f.search(query);
        let b = f.search(query);
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap(),
            "{query}"
        );
    }
    assert_eq!(hits(&f.search("elan"), "entries").len(), 1);
    let before = f.revision();
    assert!(ProjectService::apply_alias(
        &f.state,
        f.project,
        e.id,
        before,
        AliasCommand::Add { text: "   ".into() }
    )
    .is_err());
    assert!(
        ProjectService::search_project(&f.state, f.project, request(&"a".repeat(257))).is_err()
    );
    assert_eq!(f.revision(), before);
}
#[test]
fn fields_units_choices_and_canonical_relationship_context_are_searchable() {
    let f = Fixture::new();
    let a = f.entry("Aster");
    let b = f.entry("Dawn");
    for (name, kind, value, unit, options) in [
        (
            "Mass",
            FieldKind::Number,
            Some(FieldValue::Number(8000000.0)),
            Some("tons".into()),
            vec![],
        ),
        (
            "Color",
            FieldKind::Choice,
            None,
            None,
            vec!["Azure".into(), "Gold".into()],
        ),
        (
            "Description",
            FieldKind::ShortText,
            Some(FieldValue::Text("A silver observatory".into())),
            None,
            vec![],
        ),
    ] {
        let fields = ProjectService::apply_fields(
            &f.state,
            f.project,
            a.id,
            f.revision(),
            FieldCommand::Create {
                name: name.into(),
                field_kind: kind,
                unit,
                provider: FieldProvider {
                    kind: ProviderKind::Entry,
                    id: a.id.to_string(),
                },
                options,
                value,
            },
        )
        .unwrap();
        if name == "Color" {
            let d = fields
                .definitions
                .iter()
                .find(|d| d.name == "Color")
                .unwrap();
            ProjectService::apply_fields(
                &f.state,
                f.project,
                a.id,
                f.revision(),
                FieldCommand::SetValues {
                    edits: vec![FieldEdit {
                        field_id: d.id,
                        value: Some(FieldValue::Choices(vec![d.options[0].id])),
                    }],
                },
            )
            .unwrap();
        }
    }
    let d = ProjectService::apply_relationships(
        &f.state,
        f.project,
        a.id,
        f.revision(),
        RelationshipCommand::CreateDefinition {
            draft: DefinitionDraft {
                name: "Allegiance".into(),
                forward_label: "protects".into(),
                inverse_label: "protected by".into(),
                directed: true,
                expected_targets_per_source: None,
                expected_sources_per_target: None,
            },
        },
    )
    .unwrap()
    .definitions[0]
        .id;
    let r = ProjectService::apply_relationships(
        &f.state,
        f.project,
        a.id,
        f.revision(),
        RelationshipCommand::Connect {
            definition_id: d,
            perspective: Perspective::Source,
            other: OtherEntry::Existing { id: b.id },
            note: "An ancient oath".into(),
            replace: vec![],
        },
    )
    .unwrap()
    .relationships[0]
        .id;
    for query in [
        "8000000 tons",
        "azure",
        "silver observ",
        "allegiance",
        "ancient oath",
        "protected",
    ] {
        assert!(!hits(&f.search(query), "structured").is_empty(), "{query}");
    }
    assert_eq!(
        hits(&f.search("ancient"), "structured")[0].target,
        SearchTarget::Relationship {
            relationship_id: r.to_string(),
            perspective_entry_id: Some(a.id.to_string())
        }
    );
    ProjectService::update_entry_name(
        &f.state,
        f.project,
        b.id,
        b.revision,
        Some("Sunrise".into()),
    )
    .unwrap();
    assert!(hits(&f.search("Sunrise protects"), "structured")[0]
        .title
        .contains("Sunrise"));
}
#[test]
fn story_links_are_structured_but_notes_and_manuscript_are_text() {
    let f = Fixture::new();
    let e = f.entry("Sable");
    let c = f.chapter("Crossing", "Only prose mentions Sable.");
    assert!(hits(&f.search("Sable"), "structured").is_empty());
    f.story(StoryCommand::SetLink {
        chapter_id: c.chapter.id,
        entry_id: e.id,
        role_ids: vec![],
    });
    f.story(StoryCommand::Save{chapter_id:c.chapter.id,title:None,documents:vec![DocumentEdit{area:DocumentArea::Notes,schema_version:1,content:json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"A secret observatory"}]}]})}]});
    assert_eq!(hits(&f.search("Sable"), "structured").len(), 1);
    assert_eq!(
        hits(&f.search("secret"), "text")[0].target,
        SearchTarget::Chapter {
            chapter_id: c.chapter.id.to_string(),
            area: "notes".into()
        }
    );
    f.story(StoryCommand::SetState {
        chapter_id: c.chapter.id,
        state: WorkspaceState::Archived,
    });
    assert!(hits(&f.search("secret"), "text").is_empty());
    let mut request = request("secret");
    request.include_inactive = true;
    assert_eq!(
        hits(
            &ProjectService::search_project(&f.state, f.project, request).unwrap(),
            "text"
        )[0]
        .workspace_state,
        "archived"
    );
}
#[test]
fn missing_newer_and_damaged_caches_rebuild_without_changing_authored_revision() {
    let f = Fixture::new();
    let e = f.entry("Orion");
    f.alias(e.id, "Hunter");
    f.chapter("Stars", "Orion is here.");
    let expected = f.search("Orion");
    let before = f.revision();
    for damage in [
        "DROP TABLE search_index",
        "UPDATE search_index_data SET block=X'00' WHERE id>2",
        "UPDATE derived_index_state SET schema_version=999",
        "UPDATE derived_index_state SET schema_version=1",
        "UPDATE search_index SET payload='broken'",
        "DELETE FROM search_index",
        "UPDATE derived_index_state SET indexed_revision=-1",
    ] {
        f.db().execute_batch(damage).unwrap();
        assert_eq!(
            serde_json::to_value(f.search("Orion")).unwrap(),
            serde_json::to_value(&expected).unwrap(),
            "{damage}"
        );
        assert_eq!(f.revision(), before);
        assert_eq!(
            f.db()
                .query_row("SELECT dirty FROM derived_index_state", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn failed_rebuild_returns_current_source_and_never_breaks_a_save() {
    let f = Fixture::new();
    let e = f.entry("Old");
    f.search("Old");
    f.db().execute_batch("CREATE TRIGGER block_index BEFORE UPDATE OF indexed_revision ON derived_index_state BEGIN SELECT RAISE(ABORT,'injected cache write failure'); END;").unwrap();
    ProjectService::update_entry_name(&f.state, f.project, e.id, e.revision, Some("New".into()))
        .unwrap();
    assert_eq!(hits(&f.search("New"), "entries")[0].title, "New");
    assert!(hits(&f.search("Old"), "entries").is_empty());
    assert_eq!(
        f.db()
            .query_row("SELECT dirty FROM derived_index_state", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    f.alias(e.id, "Survivor");
    assert_eq!(hits(&f.search("Survivor"), "entries")[0].title, "New");
    f.db().execute_batch("DROP TRIGGER block_index").unwrap();
    f.search("New");
    assert_eq!(
        f.db()
            .query_row("SELECT dirty FROM derived_index_state", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn source_and_index_dirty_marker_roll_back_together_and_stale_alias_writes_fail() {
    let f = Fixture::new();
    let e = f.entry("Quartz");
    f.search("Quartz");
    let before = f.revision();
    let db = f.db();
    db.execute_batch("CREATE TRIGGER block_source BEFORE UPDATE OF last_committed_revision ON project_meta BEGIN SELECT RAISE(ABORT,'injected save failure'); END;").unwrap();
    assert!(ProjectService::apply_alias(
        &f.state,
        f.project,
        e.id,
        before,
        AliasCommand::Add {
            text: "Lost".into()
        }
    )
    .is_err());
    assert!(ProjectService::read_aliases(&f.state, f.project, e.id)
        .unwrap()
        .aliases
        .is_empty());
    assert_eq!(f.revision(), before);
    assert_eq!(
        db.query_row("SELECT dirty FROM derived_index_state", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER block_source").unwrap();
    f.alias(e.id, "Crystal");
    assert!(ProjectService::apply_alias(
        &f.state,
        f.project,
        e.id,
        before,
        AliasCommand::Add {
            text: "Stale".into()
        }
    )
    .is_err());
    assert_eq!(
        ProjectService::read_aliases(&f.state, f.project, e.id)
            .unwrap()
            .aliases
            .len(),
        1
    );
}
#[test]
fn reopening_and_restore_as_copy_preserve_alias_ids_and_search_scope() {
    let f = Fixture::new();
    let e = f.entry("Voyager");
    let aliases = f.alias(e.id, "Traveler");
    f.search("Traveler");
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(
        ProjectService::read_aliases(&f.state, f.project, e.id)
            .unwrap()
            .aliases,
        aliases.aliases
    );
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let copy =
        ProjectService::restore_backup_as_copy(&f.state, &backup, f.dir.path(), Some("Copy"))
            .unwrap();
    assert_ne!(copy.project_id, f.project);
    assert_eq!(
        ProjectService::read_aliases(&f.state, copy.project_id, e.id)
            .unwrap()
            .aliases,
        aliases.aliases
    );
    assert_eq!(
        hits(
            &ProjectService::search_project(&f.state, copy.project_id, request("Traveler"))
                .unwrap(),
            "entries"
        )[0]
        .target,
        SearchTarget::Entry {
            entry_id: e.id.to_string()
        }
    );
    let revision = ProjectService::read_aliases(&f.state, copy.project_id, e.id)
        .unwrap()
        .global_revision;
    ProjectService::apply_alias(
        &f.state,
        copy.project_id,
        e.id,
        revision,
        AliasCommand::Add {
            text: "Copy only".into(),
        },
    )
    .unwrap();
    assert!(hits(&f.search("Copy only"), "entries").is_empty());
    assert!(
        ProjectService::search_project(&f.state, ProjectId::new(), request("Traveler")).is_err()
    );
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}
#[test]
fn schema_nine_upgrade_is_atomic_and_keeps_a_recovery_snapshot() {
    let f = Fixture::new();
    let e = f.entry("Earlier Entry");
    ProjectService::close_project(&f.state, f.project).unwrap();
    let db = f.db();
    db.execute_batch("DROP TABLE project_pin; DROP TABLE project_recent; DROP TABLE project_navigation_settings; DROP TRIGGER rich_document_owner_insert; DROP TRIGGER rich_document_owner_update; DROP TRIGGER rich_document_keep_entry; DROP TRIGGER rich_document_keep_chapter; DROP TABLE occurrence_entry; DROP TABLE occurrence_chapter; DROP TABLE temporal_occurrence; DROP TRIGGER occurrence_event_preserve; DROP TABLE timeline_calendar; DELETE FROM capability_def WHERE id='event'; DROP TRIGGER search_source_updated; DROP TRIGGER search_source_created; DROP TABLE search_index; DROP TABLE derived_index_state; DROP TABLE entry_alias; PRAGMA user_version=9; UPDATE project_meta SET schema_version=9; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;").unwrap();
    let root = std::path::Path::new(&f.path);
    let manifest_path = root.join("manifest.json");
    let mut manifest = Manifest::read(&manifest_path).unwrap();
    manifest.schema_version = 9;
    manifest.write(&manifest_path).unwrap();
    assert!(ProjectService::open_project(&f.state, root, false).is_err());
    assert!(db.prepare("SELECT * FROM entry_alias").is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        9
    );
    db.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(db);
    let opened = ProjectService::open_project(&f.state, root, false).unwrap();
    assert_eq!(opened.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(
        hits(&f.search("Earlier"), "entries")[0].target,
        SearchTarget::Entry {
            entry_id: e.id.to_string()
        }
    );
    let recovery = Connection::open_with_flags(
        f.dir
            .path()
            .join(".worldcrafter-migration-recovery")
            .join(f.project.to_string())
            .join("schema-v9.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        9
    );
    assert_eq!(
        recovery
            .query_row("SELECT authored_name FROM entry", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "Earlier Entry"
    );
}
#[test]
fn a_thousand_entries_keep_results_bounded_and_exact_names_first() {
    let f = Fixture::new();
    let exact = f.entry("Station");
    let db = f.db();
    let tx = db.unchecked_transaction().unwrap();
    let category: String = tx
        .query_row("SELECT id FROM category LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    for n in 0..1000 {
        let id = EntryId::new().to_string();
        tx.execute(
            "INSERT INTO record_identity VALUES(?1,'entry','active',?2,?2)",
            params![id, now],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO entry VALUES(?1,?2,NULL,?3,?4,?4,1)",
            params![id, category, format!("Station {n}"), now],
        )
        .unwrap();
    }
    tx.execute(
        "UPDATE project_meta SET last_committed_revision=last_committed_revision+1",
        [],
    )
    .unwrap();
    tx.commit().unwrap();
    for _ in 0..2 {
        let r = f.search("Station");
        let g = &r.groups[0];
        assert_eq!(g.total, 1001);
        assert_eq!(g.hits.len(), 10);
        assert_eq!(
            g.hits[0].target,
            SearchTarget::Entry {
                entry_id: exact.id.to_string()
            }
        );
    }
    let mut q = request("Station");
    q.limit_per_group = 150;
    let expanded = ProjectService::search_project(&f.state, f.project, q).unwrap();
    assert_eq!(expanded.groups[0].total, 1001);
    assert_eq!(expanded.groups[0].hits.len(), 150);
}

fn connect(f: &Fixture, a: EntryId, b: EntryId, directed: bool, note: &str) -> String {
    let snapshot = ProjectService::apply_relationships(
        &f.state,
        f.project,
        a,
        f.revision(),
        RelationshipCommand::CreateDefinition {
            draft: DefinitionDraft {
                name: if directed { "Protection" } else { "Adversary" }.into(),
                forward_label: if directed { "protects" } else { "opposes" }.into(),
                inverse_label: if directed { "protected by" } else { "opposes" }.into(),
                directed,
                expected_targets_per_source: None,
                expected_sources_per_target: None,
            },
        },
    )
    .unwrap();
    ProjectService::apply_relationships(
        &f.state,
        f.project,
        a,
        f.revision(),
        RelationshipCommand::Connect {
            definition_id: snapshot.definitions.last().unwrap().id,
            perspective: Perspective::Source,
            other: OtherEntry::Existing { id: b },
            note: note.into(),
            replace: vec![],
        },
    )
    .unwrap()
    .relationships
    .last()
    .unwrap()
    .id
    .to_string()
}

#[test]
fn search_relationship_perspective_preserves_semantics_and_separates_authored_notes() {
    for directed in [true, false] {
        let f = Fixture::new();
        let a = f.entry("Aster");
        let b = f.entry("Dawn");
        f.alias(b.id, "Morning Star");
        let id = connect(&f, a.id, b.id, directed, "An old promise.");
        let expected = if directed {
            "Dawn protected by Aster"
        } else {
            "Dawn opposes Aster"
        };
        let before = f.revision();
        for query in ["Dawn", "daw", "Morning Star", "dawn promise"] {
            f.db()
                .execute("UPDATE derived_index_state SET dirty=1", [])
                .unwrap();
            for _ in 0..2 {
                let hit = hits(&f.search(query), "structured").remove(0);
                assert_eq!(hit.title, expected);
                assert_eq!(hit.excerpt, "An old promise.");
                assert_eq!(hit.preview, "An old promise.");
                assert_eq!(
                    hit.target,
                    SearchTarget::Relationship {
                        relationship_id: id.clone(),
                        perspective_entry_id: Some(b.id.to_string())
                    }
                );
            }
        }
        let hit = hits(&f.search("Aster"), "structured").remove(0);
        assert_eq!(
            hit.title,
            if directed {
                "Aster protects Dawn"
            } else {
                "Aster opposes Dawn"
            }
        );
        assert_eq!(f.revision(), before);
        let source: String = f.db().query_row("SELECT record_id FROM relationship_participant WHERE instance_id=?1 AND slot='source'", [&id], |r|r.get(0)).unwrap();
        assert_eq!(source, a.id.to_string());
    }
}

#[test]
fn entry_scope_uses_explicit_links_filters_before_limits_and_rejects_foreign_ids() {
    let f = Fixture::new();
    let a = f.entry("Aster");
    let b = f.entry("Dawn");
    connect(&f, a.id, b.id, true, "A promise.");
    let linked = f.chapter("A linked Chapter", "Aster walks through the forest.");
    let mention = f.chapter(
        "A prose mention",
        "Dawn and Aster are words here, not links.",
    );
    f.story(StoryCommand::SetLink {
        chapter_id: linked.chapter.id,
        entry_id: b.id,
        role_ids: vec![],
    });
    ProjectService::apply_fields(
        &f.state,
        f.project,
        b.id,
        f.revision(),
        FieldCommand::Create {
            name: "Age".into(),
            field_kind: FieldKind::Number,
            unit: Some("years".into()),
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: b.id.to_string(),
            },
            options: vec![],
            value: Some(FieldValue::Number(48.0)),
        },
    )
    .unwrap();
    let mut q = request("");
    q.entry_id = Some(b.id);
    q.limit_per_group = 1;
    let all = ProjectService::search_project(&f.state, f.project, q.clone()).unwrap();
    let structured = all.groups.iter().find(|g| g.kind == "structured").unwrap();
    assert_eq!(structured.total, 3);
    assert_eq!(structured.hits.len(), 1);
    assert!(
        matches!(&structured.hits[0].target, SearchTarget::Chapter { chapter_id, .. } if chapter_id == &linked.chapter.id.to_string())
    );
    for (kind, key) in [
        (StructuredKind::Fields, "field:"),
        (StructuredKind::Relationships, "relationship:"),
        (StructuredKind::Chapters, "story-link:"),
    ] {
        q.structured_kind = Some(kind);
        for _ in 0..2 {
            let r = ProjectService::search_project(&f.state, f.project, q.clone()).unwrap();
            let group = r.groups.iter().find(|g| g.kind == "structured").unwrap();
            assert_eq!(group.total, 1);
            assert!(group.hits[0].key.starts_with(key));
            assert_eq!(hits(&r, "text").len(), 1); // Empty Plan/Notes are not results.
            assert!(!r.groups.iter().flat_map(|g|&g.hits).any(|h|matches!(&h.target,SearchTarget::Chapter { chapter_id, .. } if chapter_id==&mention.chapter.id.to_string())));
        }
    }
    q.structured_kind = Some(StructuredKind::Relationships);
    q.query = "Aster".into();
    let scoped = ProjectService::search_project(&f.state, f.project, q.clone()).unwrap();
    assert_eq!(
        hits(&scoped, "structured")[0].title,
        "Dawn protected by Aster"
    );
    assert_eq!(hits(&scoped, "text")[0].title, "A linked Chapter");
    q.entry_id = Some(EntryId::new());
    assert!(ProjectService::search_project(&f.state, f.project, q).is_err());
}

#[test]
fn chapter_excerpts_are_bounded_and_requested_counts_can_exceed_one_hundred() {
    let f = Fixture::new();
    let text = format!("Dawn {}", "walks among the ancient trees. ".repeat(100));
    let c = f.chapter("Dawn crossing", &text);
    let hit = hits(&f.search("Dawn"), "text").remove(0);
    assert!(hit.preview.len() > hit.excerpt.len());
    assert!(hit.excerpt.chars().count() <= 223);
    assert!(hit.preview.chars().count() <= 1203);
    assert!(hit.preview.ends_with('…'));
    assert_eq!(
        ProjectService::read_chapter(&f.state, f.project, c.chapter.id)
            .unwrap()
            .documents
            .iter()
            .find(|d| d.area == DocumentArea::Manuscript)
            .unwrap()
            .plain_text,
        format!("{text}\n")
    );
    let mut q = request("Dawn");
    q.limit_per_group = 150;
    assert!(ProjectService::search_project(&f.state, f.project, q.clone()).is_ok());
    q.limit_per_group = 0;
    assert!(ProjectService::search_project(&f.state, f.project, q).is_err());
}

#[test]
fn preview_area_filters_before_limits_without_changing_other_groups_or_chapter_text() {
    let f = Fixture::new();
    let e = f.entry("Voyage");
    let c = f.chapter("Voyage", "Voyage manuscript");
    f.story(StoryCommand::SetLink {
        chapter_id: c.chapter.id,
        entry_id: e.id,
        role_ids: vec![],
    });
    f.story(StoryCommand::Save { chapter_id:c.chapter.id, title:None, documents: [DocumentArea::Plan,DocumentArea::Notes].into_iter().map(|area| DocumentEdit {
        area, schema_version:1, content:json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":format!("Voyage {}",area.as_str())}]}]})
    }).collect() });
    let before = f.revision();
    let mut q = request("Voyage");
    q.limit_per_group = 1;
    let all = ProjectService::search_project(&f.state, f.project, q.clone()).unwrap();
    assert_eq!(
        all.groups.iter().find(|g| g.kind == "text").unwrap().total,
        3
    );
    for area in [
        DocumentArea::Manuscript,
        DocumentArea::Plan,
        DocumentArea::Notes,
    ] {
        q.text_area = Some(area);
        f.db()
            .execute("UPDATE derived_index_state SET dirty=1", [])
            .unwrap();
        for _ in 0..2 {
            let r = ProjectService::search_project(&f.state, f.project, q.clone()).unwrap();
            let group = r.groups.iter().find(|g| g.kind == "text").unwrap();
            assert_eq!(group.total, 1);
            assert_eq!(group.hits.len(), 1);
            assert_eq!(group.hits[0].excerpt, format!("Voyage {}", area.as_str()));
            assert_eq!(
                group.hits[0].target,
                SearchTarget::Chapter {
                    chapter_id: c.chapter.id.to_string(),
                    area: area.as_str().into()
                }
            );
            for kind in ["entries", "chapters", "structured"] {
                assert_eq!(hits(&r, kind), hits(&all, kind));
            }
        }
    }
    let saved = ProjectService::read_chapter(&f.state, f.project, c.chapter.id).unwrap();
    for d in saved.documents {
        assert_eq!(d.plain_text, format!("Voyage {}\n", d.area.as_str()));
    }
    assert_eq!(f.revision(), before);
    assert!(serde_json::from_value::<SearchRequest>(
        json!({"query":"Voyage","limitPerGroup":10,"includeInactive":false,"textArea":"unknown"})
    )
    .is_err());
}

#[test]
fn custom_roles_are_findable_before_assignment_and_usage_tracks_canonical_links() {
    let f = Fixture::new();
    let person = f.entry("Traveller");
    let other = f.entry("Companion");
    let chapter = f.chapter("Opening", "Intro is just a word in this manuscript.");
    // Prime the cache before creation, then exercise its invalidation on each edit.
    assert!(hits(&f.search("Intro"), "roles").is_empty());
    let created = f.story(StoryCommand::CreateRole {
        chapter_id: chapter.chapter.id,
        name: "Intro".into(),
    });
    let intro = created
        .roles
        .iter()
        .find(|r| r.name == "Intro")
        .unwrap()
        .id
        .clone();
    let pov = created
        .roles
        .iter()
        .find(|r| r.name == "POV")
        .unwrap()
        .id
        .clone();
    let found = f.search("intro");
    assert_eq!(
        hits(&found, "roles")[0].target,
        SearchTarget::StoryRole {
            role_id: intro.clone(),
            name: "Intro".into()
        }
    );
    assert!(hits(&found, "structured").is_empty());
    assert_eq!(hits(&found, "text").len(), 1);
    let mut usage = request("");
    usage.story_role_id = Some(intro.clone());
    let query = |request: SearchRequest| {
        ProjectService::search_project(&f.state, f.project, request).unwrap()
    };
    assert!(query(usage.clone()).groups.iter().all(|g| g.total == 0));
    let linked = f.story(StoryCommand::SetLink {
        chapter_id: chapter.chapter.id,
        entry_id: person.id,
        role_ids: vec![intro.clone(), pov.clone()],
    });
    let link_id = linked.links[0].id.clone();
    f.story(StoryCommand::SetLink {
        chapter_id: chapter.chapter.id,
        entry_id: other.id,
        role_ids: vec![],
    });
    let assigned = query(usage.clone());
    assert_eq!(hits(&assigned, "structured").len(), 1);
    assert!(hits(&assigned, "text").is_empty());
    assert!(hits(&assigned, "roles").is_empty());
    assert!(hits(&assigned, "structured")[0]
        .context
        .contains("Traveller"));
    assert!(hits(&assigned, "structured")[0].context.contains("POV"));
    assert_eq!(
        serde_json::to_value(&assigned).unwrap(),
        serde_json::to_value(query(usage.clone())).unwrap()
    );
    let mut scoped = usage.clone();
    scoped.entry_id = Some(other.id);
    assert!(hits(&query(scoped), "structured").is_empty());
    let mut named = usage.clone();
    named.query = "trav".into();
    assert_eq!(hits(&query(named), "structured").len(), 1);
    let cleared = f.story(StoryCommand::SetLink {
        chapter_id: chapter.chapter.id,
        entry_id: person.id,
        role_ids: vec![],
    });
    assert_eq!(cleared.links.len(), 2);
    assert_eq!(
        cleared
            .links
            .iter()
            .find(|l| l.entry_id == Some(person.id))
            .unwrap()
            .id,
        link_id
    );
    assert!(hits(&query(usage.clone()), "structured").is_empty());
    assert_eq!(hits(&f.search("Intro"), "roles").len(), 1);
    assert_eq!(
        cleared.documents[0].plain_text,
        chapter.documents[0].plain_text
    );
    // An older derived payload is rebuilt without changing the authored revision.
    f.db()
        .execute("UPDATE derived_index_state SET schema_version=2", [])
        .unwrap();
    let before = f.revision();
    assert_eq!(hits(&f.search("Intro"), "roles").len(), 1);
    assert_eq!(f.revision(), before);
    assert_eq!(
        f.db()
            .query_row("SELECT schema_version FROM derived_index_state", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        6
    );
}

#[test]
fn role_usage_filters_by_identity_and_respects_inactive_chapters() {
    let f = Fixture::new();
    let person = f.entry("Traveller");
    let c = f.chapter("Opening", "POV Intro");
    let a = f.story(StoryCommand::CreateRole {
        chapter_id: c.chapter.id,
        name: "Intro".into(),
    });
    let role_a = a
        .roles
        .iter()
        .find(|r| r.name == "Intro")
        .unwrap()
        .id
        .clone();
    let b = f.story(StoryCommand::CreateRole {
        chapter_id: c.chapter.id,
        name: "Introduction".into(),
    });
    let role_b = b
        .roles
        .iter()
        .find(|r| r.name == "Introduction")
        .unwrap()
        .id
        .clone();
    f.story(StoryCommand::SetLink {
        chapter_id: c.chapter.id,
        entry_id: person.id,
        role_ids: vec![role_b.clone()],
    });
    let mut req = request("");
    req.story_role_id = Some(role_a);
    assert!(hits(
        &ProjectService::search_project(&f.state, f.project, req.clone()).unwrap(),
        "structured"
    )
    .is_empty());
    req.story_role_id = Some(role_b);
    assert_eq!(
        hits(
            &ProjectService::search_project(&f.state, f.project, req.clone()).unwrap(),
            "structured"
        )
        .len(),
        1
    );
    f.story(StoryCommand::SetState {
        chapter_id: c.chapter.id,
        state: WorkspaceState::Archived,
    });
    assert!(hits(
        &ProjectService::search_project(&f.state, f.project, req.clone()).unwrap(),
        "structured"
    )
    .is_empty());
    req.include_inactive = true;
    assert_eq!(
        hits(
            &ProjectService::search_project(&f.state, f.project, req.clone()).unwrap(),
            "structured"
        )[0]
        .workspace_state,
        "archived"
    );
    req.story_role_id = Some(uuid::Uuid::now_v7().to_string());
    assert!(ProjectService::search_project(&f.state, f.project, req).is_err());
}
