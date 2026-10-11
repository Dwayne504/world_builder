use rusqlite::{params, Connection};
use std::path::Path;
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        fields::*, lifecycle::StructureCommand, search::*, story::WorkspaceState,
        structure::FieldId, Entry, EntryId, ProjectId,
    },
    package::Manifest,
};
struct Fixture {
    dir: TempDir,
    state: AppState,
    project: ProjectId,
    path: String,
    owner: Entry,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let p = ProjectService::create_project(&state, dir.path(), "References fixture").unwrap();
        let owner =
            ProjectService::create_entry(&state, p.project_id, None, None, Some("Research".into()))
                .unwrap();
        Self {
            dir,
            state,
            project: p.project_id,
            path: p.package_path,
            owner,
        }
    }
    fn entry(&self, name: &str) -> Entry {
        ProjectService::create_entry(&self.state, self.project, None, None, Some(name.into()))
            .unwrap()
    }
    fn read(&self) -> EntryFields {
        ProjectService::read_fields(&self.state, self.project, self.owner.id).unwrap()
    }
    fn apply(&self, command: FieldCommand) -> EntryFields {
        ProjectService::apply_fields(
            &self.state,
            self.project,
            self.owner.id,
            self.read().global_revision,
            command,
        )
        .unwrap()
    }
    fn create(&self) -> FieldId {
        let before = self.read();
        self.apply(FieldCommand::Create {
            name: "See also".into(),
            field_kind: FieldKind::EntryReference,
            unit: None,
            provider: FieldProvider {
                kind: ProviderKind::Category,
                id: self.owner.category_id.to_string(),
            },
            options: vec![],
            value: None,
        })
        .definitions
        .iter()
        .find(|d| !before.definitions.iter().any(|old| old.id == d.id))
        .unwrap()
        .id
    }
    fn selection(&self, id: FieldId) -> ReferenceSelection {
        match self
            .read()
            .fields
            .into_iter()
            .find(|f| f.definition.id == id)
            .unwrap()
            .value
        {
            Some(FieldValue::EntryReferences(s)) => s,
            None => ReferenceSelection {
                expected_ids: vec![],
                items: vec![],
            },
            _ => panic!("reference expected"),
        }
    }
    fn set(&self, id: FieldId, items: Vec<ReferenceItem>) -> EntryFields {
        let mut s = self.selection(id);
        s.items = items;
        self.apply(set(id, s))
    }
    fn db(&self) -> Connection {
        let db = Connection::open(Path::new(&self.path).join("data/project.sqlite")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db
    }
    fn state(&self, id: EntryId, state: WorkspaceState) {
        ProjectService::apply_structure(
            &self.state,
            self.project,
            self.read().global_revision,
            StructureCommand::SetEntryState { id, state },
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
fn item(id: EntryId) -> ReferenceItem {
    ReferenceItem::Entry { entry_id: id }
}
fn set(id: FieldId, s: ReferenceSelection) -> FieldCommand {
    FieldCommand::SetValues {
        edits: vec![FieldEdit {
            field_id: id,
            value: Some(FieldValue::EntryReferences(s)),
        }],
    }
}
fn search(query: &str, entry: Option<EntryId>) -> SearchRequest {
    SearchRequest {
        query: query.into(),
        include_inactive: true,
        limit_per_group: 10,
        entry_id: entry,
        structured_kind: None,
        text_area: None,
        story_role_id: None,
    }
}

#[test]
fn empty_single_multiple_order_and_identity_survive_rename_and_inactive_targets() {
    let f = Fixture::new();
    let id = f.create();
    assert!(f.read().fields[0].value.is_none());
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM entry_reference_value", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let a = f.entry("Atlas");
    let b = f.entry("Index");
    f.set(id, vec![item(a.id)]);
    let first = f.selection(id);
    f.set(id, vec![item(a.id), item(b.id), item(f.owner.id)]);
    assert_eq!(f.selection(id).expected_ids[0], first.expected_ids[0]);
    let old = f.selection(id);
    let mut reordered = old.items.clone();
    reordered.reverse();
    f.set(id, reordered);
    let mut expected = old.expected_ids;
    expected.reverse();
    assert_eq!(f.selection(id).expected_ids, expected);
    ProjectService::update_entry_name(
        &f.state,
        f.project,
        a.id,
        a.revision,
        Some("Renamed Atlas".into()),
    )
    .unwrap();
    let cyclic = ProjectService::apply_fields(
        &f.state,
        f.project,
        a.id,
        f.read().global_revision,
        set(
            id,
            ReferenceSelection {
                expected_ids: vec![],
                items: vec![item(f.owner.id)],
            },
        ),
    );
    cyclic.unwrap();
    {
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM relationship_instance", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    f.state(a.id, WorkspaceState::Archived);
    f.state(b.id, WorkspaceState::Trashed);
    let before = f.selection(id);
    f.set(id, before.items.clone());
    assert_eq!(f.selection(id), before);
    let field = &f.read().fields[0];
    assert!(field
        .reference_targets
        .iter()
        .any(|r| r.target_id == Some(a.id.to_string())
            && r.label == "Renamed Atlas"
            && r.workspace_state.as_deref() == Some("archived")));
    f.set(id, vec![]);
    assert!(f.read().fields[0].value.is_none());
}

#[test]
fn stale_clear_reorder_recreation_duplicates_and_foreign_targets_fail_atomically() {
    let f = Fixture::new();
    let id = f.create();
    let a = f.entry("One");
    let b = f.entry("Two");
    f.set(id, vec![item(a.id), item(b.id)]);
    let old = f.selection(id);
    let before = f.read();
    let foreign = Fixture::new();
    let bad = vec![
        vec![item(a.id), item(a.id)],
        vec![item(foreign.owner.id)],
        vec![ReferenceItem::Existing {
            reference_id: "borrowed".into(),
        }],
    ];
    for items in bad {
        assert!(ProjectService::apply_fields(
            &f.state,
            f.project,
            f.owner.id,
            f.read().global_revision,
            set(
                id,
                ReferenceSelection {
                    expected_ids: old.expected_ids.clone(),
                    items
                }
            )
        )
        .is_err());
        assert_eq!(f.read().fields, before.fields);
        assert_eq!(f.read().global_revision, before.global_revision);
    }
    f.set(id, vec![]);
    f.set(id, vec![item(a.id), item(b.id)]);
    assert_ne!(f.selection(id).expected_ids, old.expected_ids);
    for items in [vec![], old.items.iter().cloned().rev().collect()] {
        assert!(ProjectService::apply_fields(
            &f.state,
            f.project,
            f.owner.id,
            f.read().global_revision,
            set(
                id,
                ReferenceSelection {
                    expected_ids: old.expected_ids.clone(),
                    items
                }
            )
        )
        .is_err());
    }
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.owner.id,
        f.read().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: None
            }]
        }
    )
    .is_err());
    f.state(a.id, WorkspaceState::Archived);
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.owner.id,
        f.read().global_revision,
        set(
            id,
            ReferenceSelection {
                expected_ids: f.selection(id).expected_ids,
                items: vec![item(a.id)]
            }
        )
    )
    .is_err());
    let before = f.read();
    f.db().execute_batch("CREATE TRIGGER injected BEFORE UPDATE ON project_meta BEGIN SELECT RAISE(ABORT,'fail commit'); END;").unwrap();
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.owner.id,
        before.global_revision,
        set(
            id,
            ReferenceSelection {
                expected_ids: f.selection(id).expected_ids,
                items: vec![]
            }
        )
    )
    .is_err());
    assert_eq!(f.read().fields, before.fields);
}

#[test]
fn unresolved_snapshot_is_preserved_and_never_repaired_by_name_and_sql_checks_hold() {
    let f = Fixture::new();
    let id = f.create();
    let target = f.entry("Former Entry");
    f.set(id, vec![item(target.id)]);
    let reference = f.selection(id).expected_ids[0].clone();
    let db = f.db();
    assert!(db
        .execute("DELETE FROM entry WHERE id=?1", [target.id.to_string()])
        .is_err());
    assert!(db
        .execute(
            "UPDATE entry_reference_value SET target_record_id=NULL,unresolved_snapshot='{}'",
            []
        )
        .is_err());
    db.execute("UPDATE entry_reference_value SET target_record_id=NULL,unresolved_snapshot=?1",[serde_json::json!({"former_id":target.id.to_string(),"kind":"entry","label":"Former Entry","deleted_at":"fixture"}).to_string()]).unwrap();
    let replacement = f.entry("Former Entry");
    f.set(
        id,
        vec![
            ReferenceItem::Existing {
                reference_id: reference.clone(),
            },
            item(replacement.id),
        ],
    );
    let refs = f.read().fields[0].reference_targets.clone();
    assert_eq!(refs[0].target_id, None);
    assert_eq!(refs[1].target_id, Some(replacement.id.to_string()));
    assert_eq!(refs[0].id, reference);
    let chapter = ProjectService::apply_story(
        &f.state,
        f.project,
        f.read().global_revision,
        worldcrafter_lib::domain::story::StoryCommand::Create {
            title: "Wrong family".into(),
        },
    )
    .unwrap();
    assert!(db.execute("UPDATE entry_reference_value SET target_record_id=?1,unresolved_snapshot=NULL WHERE id=?2",params![chapter.chapter.id.to_string(),reference]).is_err());
    let forged = serde_json::json!({"kind":"entry_references","value":{"expectedIds":[],"items":[],"label":"forged"}});
    assert!(serde_json::from_value::<FieldValue>(forged).is_err());
}

#[test]
fn hide_detach_retire_delete_and_backup_copy_preserve_the_right_values() {
    let f = Fixture::new();
    let id = f.create();
    let target = f.entry("Reference");
    f.set(id, vec![item(target.id)]);
    let other = ProjectService::apply_fields(
        &f.state,
        f.project,
        target.id,
        f.read().global_revision,
        set(
            id,
            ReferenceSelection {
                expected_ids: vec![],
                items: vec![item(f.owner.id)],
            },
        ),
    )
    .unwrap();
    let initial = f.selection(id);
    f.apply(FieldCommand::SetHidden {
        field_id: id,
        hidden: true,
    });
    f.apply(FieldCommand::Unbind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Category,
            id: f.owner.category_id.to_string(),
        },
    });
    f.apply(FieldCommand::SetRetired {
        field_id: id,
        retired: true,
    });
    assert_eq!(f.selection(id), initial);
    assert!(!f.read().fields[0].available);
    assert!(f.read().fields[0].hidden);
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert_eq!(f.selection(id), initial);
    let deleted = ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.owner.id,
        id,
        f.read().global_revision,
        &f.dir.path().join("backups"),
    )
    .unwrap();
    assert!(deleted.snapshot.fields.is_empty());
    let retained = ProjectService::read_fields(&f.state, f.project, target.id).unwrap();
    assert_eq!(retained.fields[0].value, other.fields[0].value);
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        Path::new(&deleted.backup_path),
        &f.dir.path().join("copy"),
        None,
    )
    .unwrap();
    assert_ne!(copy.project_id, f.project);
    let restored = ProjectService::read_fields(&f.state, copy.project_id, f.owner.id).unwrap();
    assert_eq!(
        restored.fields[0].value,
        Some(FieldValue::EntryReferences(initial))
    );
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
    f.apply(FieldCommand::SetRetired {
        field_id: id,
        retired: false,
    });
    let second = f.create();
    let preview = ProjectService::preview_field_merge(&f.state, f.project, id, second).unwrap();
    assert!(preview.blockers.iter().any(|b| b.contains("Reference")));
    assert!(ProjectService::merge_fields(
        &f.state,
        f.project,
        id,
        second,
        preview.global_revision,
        &f.dir.path().join("backups")
    )
    .is_err());
}

#[test]
fn references_are_searchable_with_generic_incoming_context_and_rebuildable_indexes() {
    let f = Fixture::new();
    let id = f.create();
    let target = f.entry("Archive");
    f.set(id, vec![item(target.id)]);
    ProjectService::apply_alias(
        &f.state,
        f.project,
        target.id,
        f.read().global_revision,
        AliasCommand::Add {
            text: "Library".into(),
        },
    )
    .unwrap();
    let results =
        ProjectService::search_project(&f.state, f.project, search("Library", None)).unwrap();
    let hits = &results
        .groups
        .iter()
        .find(|g| g.kind == "structured")
        .unwrap()
        .hits;
    assert!(hits.iter().any(|h| h.context == "Entry reference"
        && h.target
            == SearchTarget::Entry {
                entry_id: f.owner.id.to_string()
            }));
    let incoming =
        ProjectService::search_project(&f.state, f.project, search("", Some(target.id))).unwrap();
    assert!(incoming
        .groups
        .iter()
        .flat_map(|g| &g.hits)
        .any(|h| h.key.starts_with("reference:")));
    f.db()
        .execute("UPDATE derived_index_state SET dirty=1", [])
        .unwrap();
    let fallback =
        ProjectService::search_project(&f.state, f.project, search("Library", None)).unwrap();
    assert_eq!(
        serde_json::to_value(results).unwrap(),
        serde_json::to_value(fallback).unwrap()
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM relationship_instance", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn migration_fifteen_rolls_back_recovers_and_keeps_prior_fields() {
    let f = Fixture::new();
    f.apply(FieldCommand::Create {
        name: "Existing".into(),
        field_kind: FieldKind::Number,
        unit: Some("years".into()),
        provider: FieldProvider {
            kind: ProviderKind::Entry,
            id: f.owner.id.to_string(),
        },
        options: vec![],
        value: Some(FieldValue::Number(48.0)),
    });
    let before = f.read().fields;
    ProjectService::close_project(&f.state, f.project).unwrap();
    let db = f.db();
    db.execute_batch(include_str!("fixtures/remove_entry_references.sql"))
        .unwrap();
    db.execute_batch("PRAGMA user_version=15; UPDATE project_meta SET schema_version=15; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let root = Path::new(&f.path);
    let mut manifest = Manifest::read(&root.join("manifest.json")).unwrap();
    manifest.schema_version = 15;
    manifest.write(&root.join("manifest.json")).unwrap();
    assert!(ProjectService::open_project(&f.state, root, false).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    assert!(db.prepare("SELECT * FROM entry_reference_value").is_err());
    db.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(db);
    assert_eq!(
        ProjectService::open_project(&f.state, root, false)
            .unwrap()
            .schema_version,
        16
    );
    assert_eq!(f.read().fields, before);
    let recovery = Connection::open_with_flags(
        f.dir
            .path()
            .join(".worldcrafter-migration-recovery")
            .join(f.project.to_string())
            .join("schema-v15.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    let id = f.create();
    f.set(id, vec![item(f.owner.id)]);
}

#[test]
fn type_inheritance_and_reclassification_keep_reference_values_and_ids() {
    let f = Fixture::new();
    let category = ProjectService::create_category(&f.state, f.project, "People").unwrap();
    let parent =
        ProjectService::create_type(&f.state, f.project, category.id, None, "Human").unwrap();
    let child =
        ProjectService::create_type(&f.state, f.project, category.id, Some(parent.id), "Scholar")
            .unwrap();
    let owner = ProjectService::change_entry_structure(
        &f.state,
        f.project,
        f.owner.id,
        f.owner.revision,
        category.id,
        Some(child.id),
    )
    .unwrap();
    let catalog = ProjectService::apply_template_fields(
        &f.state,
        f.project,
        f.read().global_revision,
        FieldCommand::Create {
            name: "Sources".into(),
            field_kind: FieldKind::EntryReference,
            unit: None,
            provider: FieldProvider {
                kind: ProviderKind::Type,
                id: parent.id.to_string(),
            },
            options: vec![],
            value: None,
        },
    )
    .unwrap();
    let id = catalog.definitions[0].id;
    let target = f.entry("Book");
    f.set(id, vec![item(target.id)]);
    let before = f.selection(id);
    ProjectService::apply_structure(
        &f.state,
        f.project,
        f.read().global_revision,
        StructureCommand::SetTypeRetired {
            id: parent.id,
            retired: true,
        },
        None,
    )
    .unwrap();
    assert!(f.read().fields[0].available);
    assert_eq!(f.selection(id), before);
    ProjectService::change_entry_structure(
        &f.state,
        f.project,
        owner.id,
        owner.revision,
        category.id,
        None,
    )
    .unwrap();
    assert!(!f.read().fields[0].available);
    assert_eq!(f.selection(id), before);
    let next = f.entry("More research");
    let mut items = before.items;
    items.push(item(next.id));
    f.set(id, items);
    assert_eq!(
        f.read().fields[0].reference_targets[0].target_id,
        Some(target.id.to_string())
    );
}

#[test]
fn concurrent_reference_updates_only_acknowledge_one_selection() {
    let f = Fixture::new();
    let id = f.create();
    let a = f.entry("First");
    let b = f.entry("Second");
    let revision = f.read().global_revision;
    std::thread::scope(|scope| {
        let write = |target| {
            ProjectService::apply_fields(
                &f.state,
                f.project,
                f.owner.id,
                revision,
                set(
                    id,
                    ReferenceSelection {
                        expected_ids: vec![],
                        items: vec![item(target)],
                    },
                ),
            )
        };
        let one = scope.spawn(move || write(a.id));
        let two = scope.spawn(move || write(b.id));
        let results = [one.join().unwrap(), two.join().unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    });
    assert_eq!(f.read().global_revision, revision + 1);
    assert_eq!(f.selection(id).items.len(), 1);
}
