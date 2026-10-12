use rusqlite::{params, Connection};
use serde_json::json;
use std::path::Path;
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        fields::*, lifecycle::*, spatial::*, story::WorkspaceState, CategoryId, Entry, EntryId,
        ProjectId, TypeDef, TypeId,
    },
    package::manifest::Manifest,
};
struct Fixture {
    dir: TempDir,
    state: AppState,
    project: ProjectId,
    path: String,
    category: CategoryId,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let p = ProjectService::create_project(&state, dir.path(), "Retirement fixture").unwrap();
        let category = ProjectService::create_category(&state, p.project_id, "People")
            .unwrap()
            .id;
        Self {
            dir,
            state,
            project: p.project_id,
            path: p.package_path,
            category,
        }
    }
    fn rev(&self) -> i64 {
        ProjectService::get_summary(&self.state, self.project)
            .unwrap()
            .revision
    }
    fn kind(&self, name: &str, parent: Option<TypeId>) -> TypeDef {
        ProjectService::create_type(&self.state, self.project, self.category, parent, name).unwrap()
    }
    fn entry(&self, name: Option<&str>, kind: Option<TypeId>) -> Entry {
        ProjectService::create_entry(
            &self.state,
            self.project,
            Some(self.category),
            kind,
            name.map(str::to_owned),
        )
        .unwrap()
    }
    fn apply(&self, command: StructureCommand) -> StructureOutcome {
        ProjectService::apply_structure(&self.state, self.project, self.rev(), command, None)
            .unwrap()
    }
    fn retirement(&self, id: TypeId, retired: bool) -> StructureOutcome {
        let preview =
            ProjectService::preview_type_retirement(&self.state, self.project, id).unwrap();
        ProjectService::apply_structure(
            &self.state,
            self.project,
            preview.global_revision,
            StructureCommand::SetTypeRetired { id, retired },
            None,
        )
        .unwrap()
    }
    fn db(&self) -> Connection {
        let db = Connection::open(Path::new(&self.path).join("data/project.sqlite")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db
    }
    fn field(
        &self,
        kind: TypeId,
        name: &str,
        field_kind: FieldKind,
    ) -> worldcrafter_lib::domain::structure::FieldId {
        let catalog = ProjectService::apply_template_fields(
            &self.state,
            self.project,
            self.rev(),
            FieldCommand::Create {
                name: name.into(),
                field_kind,
                unit: None,
                provider: FieldProvider {
                    kind: ProviderKind::Type,
                    id: kind.to_string(),
                },
                options: vec![],
                value: None,
            },
        )
        .unwrap();
        catalog
            .definitions
            .iter()
            .find(|d| d.name == name)
            .unwrap()
            .id
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}
fn rich() -> FieldValue {
    FieldValue::RichText(RichFieldValue {
        schema_version: 1,
        content: Some(
            json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Leopold keeps his memories Ω","marks":[{"type":"bold"}]}]}]}),
        ),
        revision: 0,
        plain_text: String::new(),
        read_only_reason: None,
        original_json: None,
    })
}

#[test]
fn review_and_retirement_preserve_all_existing_users_descendants_values_and_inheritance() {
    let f = Fixture::new();
    let parent = f.kind("Human", None);
    let child = f.kind("Pilot", Some(parent.id));
    let grandchild = f.kind("Captain", Some(child.id));
    let same_name = f.kind("Human", None);
    let age = f.field(parent.id, "Age", FieldKind::Number);
    let history = f.field(parent.id, "History", FieldKind::RichText);
    ProjectService::apply_spatial(
        &f.state,
        f.project,
        f.rev(),
        SpatialCommand::SetDefault {
            provider: CapabilityProvider::Type { id: parent.id },
            enabled: true,
        },
    )
    .unwrap();
    let direct = f.entry(Some("Leopold"), Some(parent.id));
    let descendant = f.entry(Some("Thron"), Some(grandchild.id));
    let archived = f.entry(None, Some(parent.id));
    let trashed = f.entry(Some("Earlier Human"), Some(child.id));
    f.apply(StructureCommand::SetEntryState {
        id: archived.id,
        state: WorkspaceState::Archived,
    });
    f.apply(StructureCommand::SetEntryState {
        id: trashed.id,
        state: WorkspaceState::Trashed,
    });
    let before = ProjectService::apply_fields(
        &f.state,
        f.project,
        direct.id,
        f.rev(),
        FieldCommand::SetValues {
            edits: vec![
                FieldEdit {
                    field_id: age,
                    value: Some(FieldValue::Number(48.0)),
                },
                FieldEdit {
                    field_id: history,
                    value: Some(rich()),
                },
            ],
        },
    )
    .unwrap();
    let review = ProjectService::preview_type_retirement(&f.state, f.project, parent.id).unwrap();
    assert_eq!(review.direct_entry_count, 2);
    assert_eq!(review.inherited_entry_count, 2);
    assert_eq!(review.assigned_entries.len(), 4);
    assert_eq!(review.default_fields.len(), 2);
    assert_eq!(review.descendant_types.len(), 2);
    assert!(review
        .assigned_entries
        .iter()
        .any(|e| e.label == "[Unnamed Entry]" && e.workspace_state == "archived"));
    assert!(review
        .assigned_entries
        .iter()
        .any(|e| e.id == trashed.id && e.workspace_state == "trashed"));
    let result = f.retirement(parent.id, true);
    assert_eq!(result.global_revision, review.global_revision + 1);
    assert!(result.backup_path.is_none());
    let types = ProjectService::list_types(&f.state, f.project, f.category).unwrap();
    let retired = types.iter().find(|t| t.id == parent.id).unwrap();
    assert!(retired.retired_at.is_some());
    assert_eq!(retired.revision, parent.revision + 1);
    assert!(types
        .iter()
        .find(|t| t.id == same_name.id)
        .unwrap()
        .retired_at
        .is_none());
    assert_eq!(
        types
            .iter()
            .find(|t| t.id == child.id)
            .unwrap()
            .parent_type_id,
        Some(parent.id)
    );
    assert!(types
        .iter()
        .find(|t| t.id == child.id)
        .unwrap()
        .retired_at
        .is_none());
    let after = ProjectService::read_fields(&f.state, f.project, direct.id).unwrap();
    assert_eq!(before.fields, after.fields);
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, direct.id)
            .unwrap()
            .revision,
        direct.revision
    );
    for entry in [direct.id, descendant.id, archived.id, trashed.id] {
        assert_eq!(
            ProjectService::read_fields(&f.state, f.project, entry)
                .unwrap()
                .fields
                .len(),
            2
        );
    }
    // Active descendants remain valid new choices and retain all ancestor defaults.
    let future = f.entry(Some("A new Captain"), Some(grandchild.id));
    assert_eq!(
        ProjectService::read_fields(&f.state, f.project, future.id)
            .unwrap()
            .fields
            .len(),
        2
    );
    assert!(ProjectService::read_spatial(&f.state, f.project)
        .unwrap()
        .entries
        .iter()
        .any(|e| e.id == future.id && e.spatial));
    let new_grandchild = f.kind("Navigator", Some(child.id));
    assert_eq!(new_grandchild.parent_type_id, Some(child.id));
    let renamed = ProjectService::update_entry_name(
        &f.state,
        f.project,
        direct.id,
        direct.revision,
        Some("Leopold II".into()),
    )
    .unwrap();
    let unchanged = ProjectService::change_entry_structure(
        &f.state,
        f.project,
        direct.id,
        renamed.revision,
        f.category,
        Some(parent.id),
    )
    .unwrap();
    assert_eq!(unchanged.type_id, Some(parent.id));
    f.apply(StructureCommand::SetEntryState {
        id: archived.id,
        state: WorkspaceState::Active,
    });
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, archived.id)
            .unwrap()
            .type_id,
        Some(parent.id)
    );
}

#[test]
fn every_new_typed_assignment_is_rejected_atomically_but_untyped_and_existing_uses_work() {
    let f = Fixture::new();
    let kind = f.kind("Human", None);
    let retired_user = f.entry(Some("Existing"), Some(kind.id));
    let untyped = f.entry(Some("Unclassified"), None);
    let other = f.kind("Other", None);
    ProjectService::apply_spatial(
        &f.state,
        f.project,
        f.rev(),
        SpatialCommand::SetEnabled {
            entry_id: untyped.id,
            enabled: true,
        },
    )
    .unwrap();
    f.retirement(kind.id, true);
    let revision = f.rev();
    let before = ProjectService::list_entries(&f.state, f.project).unwrap();
    assert!(ProjectService::create_entry(
        &f.state,
        f.project,
        Some(f.category),
        Some(kind.id),
        Some("Should fail".into())
    )
    .unwrap_err()
    .to_string()
    .contains("retired"));
    assert!(ProjectService::change_entry_structure(
        &f.state,
        f.project,
        untyped.id,
        untyped.revision,
        f.category,
        Some(kind.id)
    )
    .is_err());
    assert!(ProjectService::create_type(
        &f.state,
        f.project,
        f.category,
        Some(kind.id),
        "New child"
    )
    .is_err());
    assert!(ProjectService::apply_spatial(
        &f.state,
        f.project,
        revision,
        SpatialCommand::CreateChild {
            parent_id: untyped.id,
            name: Some("Rejected place".into()),
            category_id: Some(f.category),
            type_id: Some(kind.id)
        }
    )
    .is_err());
    assert_eq!(f.rev(), revision);
    assert_eq!(
        ProjectService::list_entries(&f.state, f.project).unwrap(),
        before
    );
    let db = f.db();
    // Direct SQL attempts exercise the safety net used by any future creation path.
    assert!(db
        .execute(
            "UPDATE entry SET type_id=?1 WHERE id=?2",
            params![kind.id.to_string(), untyped.id.to_string()]
        )
        .is_err());
    assert!(db
        .execute(
            "UPDATE type_def SET parent_type_id=?1 WHERE id=?2",
            params![kind.id.to_string(), other.id.to_string()]
        )
        .is_err());
    let tx = db.unchecked_transaction().unwrap();
    let id = EntryId::new().to_string();
    tx.execute("INSERT INTO record_identity(record_id,kind,workspace_state,lifecycle_changed_at,created_at) VALUES(?1,'entry','active','now','now')",[&id]).unwrap();
    assert!(tx.execute("INSERT INTO entry(id,category_id,type_id,created_at,updated_at,revision) VALUES(?1,?2,?3,'now','now',1)",params![id,f.category.to_string(),kind.id.to_string()]).is_err());
    tx.rollback().unwrap();
    assert!(db.execute("INSERT INTO type_def(id,category_id,parent_type_id,name,created_at,updated_at,revision) VALUES(?1,?2,?3,'Bypass','now','now',1)",params![TypeId::new().to_string(),f.category.to_string(),kind.id.to_string()]).is_err());
    assert_eq!(f.rev(), revision);
    assert_eq!(
        ProjectService::list_entries(&f.state, f.project).unwrap(),
        before
    );
    // Clearing is explicit; picking the retired Type again is a new assignment.
    let cleared = ProjectService::change_entry_structure(
        &f.state,
        f.project,
        retired_user.id,
        retired_user.revision,
        f.category,
        None,
    )
    .unwrap();
    assert!(ProjectService::change_entry_structure(
        &f.state,
        f.project,
        retired_user.id,
        cleared.revision,
        f.category,
        Some(kind.id)
    )
    .is_err());
    assert!(f.entry(None, None).type_id.is_none());
}

#[test]
fn stale_foreign_missing_and_failed_retirement_preserve_state_and_can_retry() {
    let f = Fixture::new();
    let kind = f.kind("Human", None);
    let review = ProjectService::preview_type_retirement(&f.state, f.project, kind.id).unwrap();
    f.entry(Some("New use after review"), Some(kind.id));
    assert_eq!(
        ProjectService::apply_structure(
            &f.state,
            f.project,
            review.global_revision,
            StructureCommand::SetTypeRetired {
                id: kind.id,
                retired: true
            },
            None
        )
        .unwrap_err()
        .kind(),
        "revision_conflict"
    );
    let foreign = Fixture::new();
    let foreign_kind = foreign.kind("Human", None);
    let revision = f.rev();
    for id in [foreign_kind.id, TypeId::new()] {
        assert!(ProjectService::preview_type_retirement(&f.state, f.project, id).is_err());
        assert!(ProjectService::apply_structure(
            &f.state,
            f.project,
            revision,
            StructureCommand::SetTypeRetired { id, retired: true },
            None
        )
        .is_err());
    }
    f.db().execute_batch("CREATE TRIGGER fail_retirement BEFORE UPDATE OF last_committed_revision ON project_meta BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(ProjectService::apply_structure(
        &f.state,
        f.project,
        revision,
        StructureCommand::SetTypeRetired {
            id: kind.id,
            retired: true
        },
        None
    )
    .is_err());
    assert_eq!(f.rev(), revision);
    assert!(
        ProjectService::list_types(&f.state, f.project, f.category).unwrap()[0]
            .retired_at
            .is_none()
    );
    f.db()
        .execute_batch("DROP TRIGGER fail_retirement")
        .unwrap();
    f.retirement(kind.id, true);
    let review = ProjectService::preview_type_retirement(&f.state, f.project, kind.id).unwrap();
    f.apply(StructureCommand::RenameType {
        id: kind.id,
        name: "Renamed Human".into(),
    });
    assert_eq!(
        ProjectService::apply_structure(
            &f.state,
            f.project,
            review.global_revision,
            StructureCommand::SetTypeRetired {
                id: kind.id,
                retired: false
            },
            None
        )
        .unwrap_err()
        .kind(),
        "revision_conflict"
    );
    f.retirement(kind.id, false);
    assert!(f.entry(None, Some(kind.id)).type_id.is_some());
}

#[test]
fn retirement_and_rich_values_survive_reopen_and_restore_as_copy_then_restore_independently() {
    let f = Fixture::new();
    let parent = f.kind("Human", None);
    let child = f.kind("Pilot", Some(parent.id));
    let history = f.field(parent.id, "History", FieldKind::RichText);
    let entry = f.entry(Some("Leopold"), Some(child.id));
    let before = ProjectService::apply_fields(
        &f.state,
        f.project,
        entry.id,
        f.rev(),
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: history,
                value: Some(rich()),
            }],
        },
    )
    .unwrap();
    f.retirement(parent.id, true);
    let retired = ProjectService::list_types(&f.state, f.project, f.category)
        .unwrap()
        .into_iter()
        .find(|t| t.id == parent.id)
        .unwrap();
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        Path::new(&backup),
        f.dir.path(),
        Some("Restored copy"),
    )
    .unwrap();
    assert_ne!(copy.project_id, f.project);
    assert_eq!(
        ProjectService::list_types(&f.state, copy.project_id, f.category)
            .unwrap()
            .into_iter()
            .find(|t| t.id == parent.id)
            .unwrap()
            .retired_at,
        retired.retired_at
    );
    assert_eq!(
        ProjectService::read_fields(&f.state, copy.project_id, entry.id)
            .unwrap()
            .fields,
        before.fields
    );
    assert!(ProjectService::create_entry(
        &f.state,
        copy.project_id,
        Some(f.category),
        Some(parent.id),
        None
    )
    .is_err());
    assert!(ProjectService::create_entry(
        &f.state,
        copy.project_id,
        Some(f.category),
        Some(child.id),
        None
    )
    .is_ok());
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert_eq!(
        ProjectService::list_types(&f.state, f.project, f.category)
            .unwrap()
            .into_iter()
            .find(|t| t.id == parent.id)
            .unwrap()
            .retired_at,
        retired.retired_at
    );
    f.retirement(parent.id, false);
    assert!(f.entry(None, Some(parent.id)).type_id.is_some());
    assert!(ProjectService::create_type(
        &f.state,
        f.project,
        f.category,
        Some(parent.id),
        "New restored child"
    )
    .is_ok());
    assert_eq!(
        ProjectService::read_fields(&f.state, f.project, entry.id)
            .unwrap()
            .fields,
        before.fields
    );
    assert!(
        ProjectService::list_types(&f.state, copy.project_id, f.category)
            .unwrap()
            .into_iter()
            .find(|t| t.id == parent.id)
            .unwrap()
            .retired_at
            .is_some()
    );
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}

#[test]
fn schema_fourteen_upgrade_is_atomic_recoverable_and_keeps_assignments_and_values() {
    let f = Fixture::new();
    let kind = f.kind("Human", None);
    let field = f.field(kind.id, "History", FieldKind::RichText);
    let entry = f.entry(Some("Leopold"), Some(kind.id));
    let fields = ProjectService::apply_fields(
        &f.state,
        f.project,
        entry.id,
        f.rev(),
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: field,
                value: Some(rich()),
            }],
        },
    )
    .unwrap();
    ProjectService::close_project(&f.state, f.project).unwrap();
    let db = f.db();
    db.execute_batch(include_str!("fixtures/remove_entry_references.sql"))
        .unwrap();
    db.execute_batch(include_str!("fixtures/remove_type_retirement.sql"))
        .unwrap();
    db.execute_batch("PRAGMA user_version=14; UPDATE project_meta SET schema_version=14; CREATE TRIGGER fail_retirement_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;").unwrap();
    let root = Path::new(&f.path);
    let mut manifest = Manifest::read(&root.join("manifest.json")).unwrap();
    manifest.schema_version = 14;
    manifest.write(&root.join("manifest.json")).unwrap();
    assert!(ProjectService::open_project(&f.state, root, false).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        14
    );
    assert!(db.prepare("SELECT retired_at FROM type_def").is_err());
    assert_eq!(
        db.query_row(
            "SELECT type_id FROM entry WHERE id=?1",
            [entry.id.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        kind.id.to_string()
    );
    db.execute_batch("DROP TRIGGER fail_retirement_upgrade")
        .unwrap();
    drop(db);
    let open = ProjectService::open_project(&f.state, root, false).unwrap();
    assert_eq!(
        open.schema_version,
        worldcrafter_lib::persistence::migrations::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, entry.id)
            .unwrap()
            .type_id,
        Some(kind.id)
    );
    assert_eq!(
        ProjectService::read_fields(&f.state, f.project, entry.id)
            .unwrap()
            .fields,
        fields.fields
    );
    assert!(
        ProjectService::list_types(&f.state, f.project, f.category).unwrap()[0]
            .retired_at
            .is_none()
    );
    let recovery = Connection::open_with_flags(
        f.dir
            .path()
            .join(".worldcrafter-migration-recovery")
            .join(f.project.to_string())
            .join("schema-v14.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        14
    );
    assert!(recovery.prepare("SELECT retired_at FROM type_def").is_err());
    f.retirement(kind.id, true);
    assert!(ProjectService::create_entry(
        &f.state,
        f.project,
        Some(f.category),
        Some(kind.id),
        None
    )
    .is_err());
}

#[test]
fn concurrent_retire_and_restore_review_cannot_both_commit() {
    let f = Fixture::new();
    let kind = f.kind("Human", None);
    let revision = f.rev();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            ProjectService::apply_structure(
                &f.state,
                f.project,
                revision,
                StructureCommand::SetTypeRetired {
                    id: kind.id,
                    retired: true,
                },
                None,
            )
        });
        let b = scope.spawn(|| {
            ProjectService::apply_structure(
                &f.state,
                f.project,
                revision,
                StructureCommand::SetTypeRetired {
                    id: kind.id,
                    retired: true,
                },
                None,
            )
        });
        let results = [a.join().unwrap(), b.join().unwrap()];
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| r.as_ref().is_err_and(|e| e.kind() == "revision_conflict"))
                .count(),
            1
        );
    });
    assert_eq!(f.rev(), revision + 1);
}
