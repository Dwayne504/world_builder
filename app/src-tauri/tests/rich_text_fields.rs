use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{fields::*, structure::FieldId, Entry, ProjectId},
};

struct Fixture {
    dir: TempDir,
    state: AppState,
    project: ProjectId,
    entry: Entry,
    path: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let project = ProjectService::create_project(&state, dir.path(), "Fields Test").unwrap();
        let entry = ProjectService::create_entry(
            &state,
            project.project_id,
            None,
            None,
            Some("Thron".into()),
        )
        .unwrap();
        Self {
            dir,
            state,
            project: project.project_id,
            entry,
            path: project.package_path,
        }
    }
    fn db(&self) -> Connection {
        let conn =
            Connection::open(std::path::Path::new(&self.path).join("data/project.sqlite")).unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn
    }
    fn rich(&self, id: FieldId) -> RichFieldValue {
        match self
            .read()
            .fields
            .iter()
            .find(|f| f.definition.id == id)
            .unwrap()
            .value
            .clone()
            .unwrap()
        {
            FieldValue::RichText(v) => v,
            _ => panic!("expected Rich Text"),
        }
    }
    fn write(&self, id: FieldId, content: serde_json::Value) {
        let revision = self
            .read()
            .fields
            .iter()
            .find(|f| f.definition.id == id)
            .and_then(|f| f.value.as_ref())
            .map(|v| match v {
                FieldValue::RichText(v) => v.revision,
                _ => 0,
            })
            .unwrap_or(0);
        self.set(id, Some(rich(content, revision)));
    }
    fn read(&self) -> EntryFields {
        ProjectService::read_fields(&self.state, self.project, self.entry.id).unwrap()
    }
    fn apply(&self, command: FieldCommand) -> EntryFields {
        ProjectService::apply_fields(
            &self.state,
            self.project,
            self.entry.id,
            self.read().global_revision,
            command,
        )
        .unwrap()
    }
    fn create(&self, kind: FieldKind, value: Option<FieldValue>) -> FieldId {
        let prior = self.read();
        let result = self.apply(FieldCommand::Create {
            unit: None,
            name: "A field".into(),
            field_kind: kind,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: self.entry.id.to_string(),
            },
            options: vec![],
            value,
        });
        result
            .definitions
            .iter()
            .find(|d| !prior.definitions.iter().any(|old| old.id == d.id))
            .unwrap()
            .id
    }
    fn set(&self, id: FieldId, value: Option<FieldValue>) -> EntryFields {
        self.apply(FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value,
            }],
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}

use serde_json::{json, Value};
use worldcrafter_lib::domain::search::{SearchRequest, StructuredKind};
fn rich(content: Value, revision: i64) -> FieldValue {
    FieldValue::RichText(RichFieldValue {
        schema_version: 1,
        content: Some(content),
        revision,
        plain_text: "untrusted submitted cache".into(),
        read_only_reason: None,
        original_json: None,
    })
}
fn prose(text: &str) -> Value {
    json!({"type":"doc","content":[{"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"Background"}]},{"type":"paragraph","content":[{"type":"text","text":text,"marks":[{"type":"bold"}]}]},{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Invented detail"}]}]}]}]})
}
#[test]
fn rich_values_are_optional_typed_versioned_and_survive_reopen_restore() {
    let f = Fixture::new();
    let id = f.create(FieldKind::RichText, None);
    assert!(f.read().fields[0].value.is_none());
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM rich_document", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let content = prose("Invented explorer");
    f.write(id, content.clone());
    assert_eq!(f.rich(id).content, Some(content));
    assert!(!f.rich(id).plain_text.contains("untrusted"));
    let before = f.read();
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(f.read(), before);
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        &backup,
        &f.dir.path().join("restore"),
        None,
    )
    .unwrap();
    assert_eq!(
        ProjectService::read_fields(&f.state, copy.project_id, f.entry.id)
            .unwrap()
            .fields,
        before.fields
    );
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}
#[test]
fn category_and_type_defaults_detach_without_losing_writing() {
    let f = Fixture::new();
    let ty = ProjectService::create_type(
        &f.state,
        f.project,
        f.entry.category_id,
        None,
        "Invented Type",
    )
    .unwrap();
    ProjectService::change_entry_structure(
        &f.state,
        f.project,
        f.entry.id,
        f.entry.revision,
        f.entry.category_id,
        Some(ty.id),
    )
    .unwrap();
    for provider in [
        FieldProvider {
            kind: ProviderKind::Category,
            id: f.entry.category_id.to_string(),
        },
        FieldProvider {
            kind: ProviderKind::Type,
            id: ty.id.to_string(),
        },
    ] {
        let before = f.read();
        let catalog = ProjectService::apply_template_fields(
            &f.state,
            f.project,
            before.global_revision,
            FieldCommand::Create {
                name: "Lore".into(),
                field_kind: FieldKind::RichText,
                unit: None,
                provider: provider.clone(),
                options: vec![],
                value: None,
            },
        )
        .unwrap();
        let id = catalog
            .definitions
            .iter()
            .find(|d| !before.definitions.iter().any(|p| p.id == d.id))
            .unwrap()
            .id;
        assert!(f
            .read()
            .fields
            .iter()
            .find(|v| v.definition.id == id)
            .unwrap()
            .value
            .is_none());
        f.write(id, prose("Template writing"));
        let authored = f.rich(id);
        f.apply(FieldCommand::Unbind {
            field_id: id,
            provider,
        });
        f.apply(FieldCommand::SetRetired {
            field_id: id,
            retired: true,
        });
        f.apply(FieldCommand::SetHidden {
            field_id: id,
            hidden: true,
        });
        assert_eq!(f.rich(id), authored);
    }
    let entry = ProjectService::get_entry(&f.state, f.project, f.entry.id).unwrap();
    ProjectService::change_entry_structure(
        &f.state,
        f.project,
        f.entry.id,
        entry.revision,
        f.entry.category_id,
        None,
    )
    .unwrap();
    assert_eq!(f.read().fields.len(), 2);
}
#[test]
fn rich_search_is_faithful_and_not_duplicated_as_description() {
    let f = Fixture::new();
    let id = f.create(FieldKind::RichText, None);
    f.write(id, prose("Zephyrlight"));
    let search = |query: &str| {
        ProjectService::search_project(
            &f.state,
            f.project,
            SearchRequest {
                query: query.into(),
                include_inactive: false,
                limit_per_group: 20,
                entry_id: Some(f.entry.id),
                structured_kind: Some(StructuredKind::Fields),
                text_area: None,
                story_role_id: None,
            },
        )
        .unwrap()
    };
    let result = search("Zephyrlight");
    let hits: Vec<_> = result.groups.iter().flat_map(|g| &g.hits).collect();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].context.contains("Rich Text"));
    assert!(search("untrusted submitted cache")
        .groups
        .iter()
        .all(|g| g.hits.is_empty()));
    f.write(id, prose("Replacement"));
    assert!(search("Zephyrlight")
        .groups
        .iter()
        .all(|g| g.hits.is_empty()));
}
#[test]
fn invalid_stale_and_failed_edits_are_atomic() {
    let f = Fixture::new();
    let id = f.create(FieldKind::RichText, None);
    f.write(id, prose("Original"));
    let old = f.rich(id);
    f.write(id, prose("Newer writing"));
    let before = f.read();
    let mut future = f.rich(id);
    future.schema_version = 99;
    for value in [
        rich(prose("Stale"), old.revision),
        rich(
            json!({"type":"doc","content":[{"type":"image","attrs":{"src":"bad"}}]}),
            f.rich(id).revision,
        ),
        FieldValue::Text("Wrong kind".into()),
        FieldValue::RichText(future),
    ] {
        assert!(ProjectService::apply_fields(
            &f.state,
            f.project,
            f.entry.id,
            before.global_revision,
            FieldCommand::SetValues {
                edits: vec![FieldEdit {
                    field_id: id,
                    value: Some(value)
                }]
            }
        )
        .is_err());
        assert_eq!(f.read(), before);
    }
    f.db().execute_batch("CREATE TRIGGER fail_rich BEFORE UPDATE OF last_committed_revision ON project_meta BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(rich(prose("Interrupted"), f.rich(id).revision))
            }]
        }
    )
    .is_err());
    assert_eq!(f.read(), before);
}
#[test]
fn unsupported_saved_documents_block_edit_clear_delete_but_allow_hide() {
    for (version, raw) in [
        (99, prose("Future").to_string()),
        (1, "damaged JSON".into()),
    ] {
        let f = Fixture::new();
        let id = f.create(FieldKind::RichText, None);
        f.write(id, prose("Saved"));
        f.db()
            .execute(
                "UPDATE rich_document SET document_schema_version=?1,canonical_json=?2",
                params![version, raw],
            )
            .unwrap();
        let held = f.rich(id);
        assert!(held.read_only_reason.is_some());
        assert_eq!(held.original_json.as_deref(), Some(raw.as_str()));
        assert!(held.content.is_none());
        for value in [
            None,
            Some(rich(prose("Overwrite"), held.revision)),
            Some(rich(json!({"type":"doc","content":[]}), held.revision)),
        ] {
            assert!(ProjectService::apply_fields(
                &f.state,
                f.project,
                f.entry.id,
                f.read().global_revision,
                FieldCommand::SetValues {
                    edits: vec![FieldEdit {
                        field_id: id,
                        value
                    }]
                }
            )
            .is_err());
        }
        assert!(ProjectService::delete_entry_field(
            &f.state,
            f.project,
            f.entry.id,
            id,
            f.read().global_revision,
            &f.dir.path().join("backups")
        )
        .is_err());
        f.apply(FieldCommand::SetHidden {
            field_id: id,
            hidden: true,
        });
        assert_eq!(f.rich(id), held);
    }
}
#[test]
fn clearing_uses_revision_zero_for_new_typing_without_reusing_old_revision() {
    let f = Fixture::new();
    let id = f.create(FieldKind::RichText, None);
    f.write(id, prose("Old"));
    let old = f.rich(id);
    f.write(id, json!({"type":"doc","content":[{"type":"paragraph"}]}));
    assert!(f.read().fields[0].value.is_none());
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM rich_document", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    f.set(id, Some(rich(prose("New document"), 0)));
    assert!(f.rich(id).revision > old.revision);
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        f.read().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(rich(prose("Old writer"), old.revision))
            }]
        }
    )
    .is_err());
    f.write(
        id,
        json!({"type":"doc","content":[{"type":"horizontalRule"}]}),
    );
    assert!(f.rich(id).content.is_some());
}
#[test]
fn delete_is_local_backed_up_and_rich_merges_are_blocked() {
    let f = Fixture::new();
    let id = f.create(FieldKind::RichText, None);
    f.write(id, prose("Keep in backup"));
    let other = f.create(FieldKind::RichText, None);
    f.write(other, prose("Other writing"));
    let preview = ProjectService::preview_field_merge(&f.state, f.project, id, other).unwrap();
    assert!(preview.blockers.iter().any(|b| b.contains("Rich Text")));
    assert!(ProjectService::merge_fields(
        &f.state,
        f.project,
        id,
        other,
        preview.global_revision,
        &f.dir.path().join("backups")
    )
    .is_err());
    let outcome = ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        id,
        f.read().global_revision,
        &f.dir.path().join("backups"),
    )
    .unwrap();
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        std::path::Path::new(&outcome.backup_path),
        &f.dir.path().join("deleted-restore"),
        None,
    )
    .unwrap();
    let restored = ProjectService::read_fields(&f.state, copy.project_id, f.entry.id).unwrap();
    assert!(restored.fields.iter().any(|field| field.definition.id==id && matches!(&field.value,Some(FieldValue::RichText(value)) if value.plain_text.contains("Keep in backup"))));
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
    assert_eq!(outcome.snapshot.fields.len(), 1);
    assert!(f.rich(other).plain_text.contains("Other writing"));
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM rich_document", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn document_ownership_and_scalar_kinds_remain_enforced() {
    let f = Fixture::new();
    let id = f.create(FieldKind::RichText, None);
    f.write(id, prose("Owned"));
    let other = f.create(
        FieldKind::ShortText,
        Some(FieldValue::Text("Existing Description".into())),
    );
    assert!(f
        .db()
        .execute(
            "UPDATE field_value SET field_id=?1 WHERE field_id=?2",
            params![other.to_string(), id.to_string()]
        )
        .is_err());
    assert!(f
        .db()
        .execute("UPDATE rich_document SET area='description'", [])
        .is_err());
    assert!(f
        .db()
        .execute(
            "UPDATE field_definition SET value_kind='rich_text' WHERE id=?1",
            [other.to_string()]
        )
        .is_err());
    assert_eq!(
        f.read()
            .fields
            .iter()
            .find(|v| v.definition.id == other)
            .unwrap()
            .value,
        Some(FieldValue::Text("Existing Description".into()))
    );
}

#[test]
fn concurrent_writers_reject_one_stale_edit_and_inactive_or_wrong_project_writes_fail() {
    use worldcrafter_lib::domain::{lifecycle::StructureCommand, story::WorkspaceState};
    let f = Fixture::new();
    let id = f.create(FieldKind::RichText, None);
    f.write(id, prose("Original"));
    let global = f.read().global_revision;
    let document = f.rich(id).revision;
    let results = std::thread::scope(|scope| {
        let threads = ["One", "Two"].map(|text| {
            let fixture = &f;
            scope.spawn(move || {
                ProjectService::apply_fields(
                    &fixture.state,
                    fixture.project,
                    fixture.entry.id,
                    global,
                    FieldCommand::SetValues {
                        edits: vec![FieldEdit {
                            field_id: id,
                            value: Some(rich(prose(text), document)),
                        }],
                    },
                )
            })
        });
        threads.map(|thread| thread.join().unwrap())
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .find_map(|result| result.as_ref().err())
            .unwrap()
            .kind(),
        "revision_conflict"
    );
    let before = f.rich(id);
    let other = Fixture::new();
    assert!(ProjectService::apply_fields(
        &other.state,
        other.project,
        other.entry.id,
        other.read().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(rich(prose("Wrong Project"), before.revision))
            }]
        }
    )
    .is_err());
    ProjectService::apply_structure(
        &f.state,
        f.project,
        f.read().global_revision,
        StructureCommand::SetEntryState {
            id: f.entry.id,
            state: WorkspaceState::Archived,
        },
        None,
    )
    .unwrap();
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        f.read().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(rich(prose("Inactive write"), before.revision))
            }]
        }
    )
    .is_err());
    assert_eq!(f.rich(id), before);
}
