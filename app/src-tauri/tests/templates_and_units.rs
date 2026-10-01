use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{fields::*, structure::FieldId, CategoryId, ProjectId},
    package::Manifest,
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
        let p = ProjectService::create_project(&state, dir.path(), "Templates test").unwrap();
        let category = ProjectService::create_category(&state, p.project_id, "Weapons").unwrap();
        Self {
            dir,
            state,
            project: p.project_id,
            path: p.package_path,
            category: category.id,
        }
    }
    fn catalog(&self) -> FieldCatalog {
        ProjectService::read_field_catalog(&self.state, self.project).unwrap()
    }
    fn apply(&self, command: FieldCommand) -> FieldCatalog {
        ProjectService::apply_template_fields(
            &self.state,
            self.project,
            self.catalog().global_revision,
            command,
        )
        .unwrap()
    }
    fn provider(&self) -> FieldProvider {
        FieldProvider {
            kind: ProviderKind::Category,
            id: self.category.to_string(),
        }
    }
    fn field(&self, unit: Option<&str>) -> FieldId {
        self.apply(FieldCommand::Create {
            name: "Mass".into(),
            field_kind: FieldKind::Number,
            unit: unit.map(String::from),
            provider: self.provider(),
            options: vec![],
            value: None,
        })
        .definitions[0]
            .id
    }
    fn db(&self) -> Connection {
        let conn =
            Connection::open(std::path::Path::new(&self.path).join("data/project.sqlite")).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}

#[test]
fn category_defaults_exist_without_entries_and_never_allocate_empty_values() {
    let f = Fixture::new();
    let id = f.field(Some(" tons "));
    assert!(ProjectService::list_entries(&f.state, f.project)
        .unwrap()
        .is_empty());
    let conn = f.db();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM field_value", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let a = ProjectService::create_entry(
        &f.state,
        f.project,
        Some(f.category),
        None,
        Some("Blade".into()),
    )
    .unwrap();
    let b =
        ProjectService::create_entry(&f.state, f.project, Some(f.category), None, None).unwrap();
    let snapshot = ProjectService::read_fields(&f.state, f.project, a.id).unwrap();
    assert_eq!(snapshot.fields[0].definition.id, id);
    assert_eq!(snapshot.fields[0].definition.unit.as_deref(), Some("tons"));
    assert_eq!(snapshot.fields[0].value, None);
    ProjectService::apply_fields(
        &f.state,
        f.project,
        a.id,
        snapshot.global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(FieldValue::Number(8_000_000.0)),
            }],
        },
    )
    .unwrap();
    assert_eq!(
        ProjectService::read_fields(&f.state, f.project, b.id)
            .unwrap()
            .fields[0]
            .value,
        None
    );
    f.apply(FieldCommand::Unbind {
        field_id: id,
        provider: f.provider(),
    });
    let retained = ProjectService::read_fields(&f.state, f.project, a.id).unwrap();
    assert!(!retained.fields[0].available);
    assert_eq!(
        retained.fields[0].value,
        Some(FieldValue::Number(8_000_000.0))
    );
    assert_eq!(retained.fields[0].definition.unit.as_deref(), Some("tons"));
    assert!(ProjectService::read_fields(&f.state, f.project, b.id)
        .unwrap()
        .fields
        .is_empty());
    f.apply(FieldCommand::Bind {
        field_id: id,
        provider: f.provider(),
    });
    assert!(
        ProjectService::read_fields(&f.state, f.project, b.id)
            .unwrap()
            .fields[0]
            .available
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM field_value", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(conn
        .execute(
            "UPDATE field_definition SET unit='kg' WHERE id=?1",
            [id.to_string()]
        )
        .is_err());
}

#[test]
fn type_defaults_merge_with_category_defaults_and_keep_shared_identity() {
    let f = Fixture::new();
    let mass = f.field(Some("tons"));
    let parent =
        ProjectService::create_type(&f.state, f.project, f.category, None, "Sword").unwrap();
    let child = ProjectService::create_type(
        &f.state,
        f.project,
        f.category,
        Some(parent.id),
        "Longsword",
    )
    .unwrap();
    let p = FieldProvider {
        kind: ProviderKind::Type,
        id: parent.id.to_string(),
    };
    let snapshot = f.apply(FieldCommand::Create {
        name: "Price".into(),
        field_kind: FieldKind::Number,
        unit: Some("gold crowns".into()),
        provider: p.clone(),
        options: vec![],
        value: None,
    });
    let price = snapshot
        .definitions
        .iter()
        .find(|d| d.name == "Price")
        .unwrap()
        .id;
    // Supplying the same definition from two templates must not duplicate it.
    f.apply(FieldCommand::Bind {
        field_id: mass,
        provider: p,
    });
    let entry =
        ProjectService::create_entry(&f.state, f.project, Some(f.category), Some(child.id), None)
            .unwrap();
    let fields = ProjectService::read_fields(&f.state, f.project, entry.id).unwrap();
    assert_eq!(fields.fields.len(), 2);
    assert!(fields.fields.iter().any(|v| v.definition.id == price));
    assert!(fields.fields.iter().all(|f| f.value.is_none()));
}

#[test]
fn template_failures_reject_foreign_providers_and_cannot_write_authored_values() {
    let f = Fixture::new();
    let before = f.catalog();
    for (kind, provider, value, unit) in [
        (
            FieldKind::Number,
            FieldProvider {
                kind: ProviderKind::Entry,
                id: "not-an-entry".into(),
            },
            None,
            None,
        ),
        (
            FieldKind::Number,
            FieldProvider {
                kind: ProviderKind::Category,
                id: CategoryId::new().to_string(),
            },
            None,
            None,
        ),
        (
            FieldKind::Number,
            f.provider(),
            Some(FieldValue::Number(2.0)),
            None,
        ),
        (
            FieldKind::ShortText,
            f.provider(),
            None,
            Some("tons".into()),
        ),
    ] {
        assert!(ProjectService::apply_template_fields(
            &f.state,
            f.project,
            before.global_revision,
            FieldCommand::Create {
                name: "Test".into(),
                field_kind: kind,
                unit,
                provider,
                options: vec![],
                value
            }
        )
        .is_err());
        assert_eq!(f.catalog(), before);
    }
    assert!(ProjectService::apply_template_fields(
        &f.state,
        f.project,
        before.global_revision,
        FieldCommand::SetValues { edits: vec![] }
    )
    .is_err());
    let conn = f.db();
    conn.execute_batch("CREATE TRIGGER fail_bind BEFORE INSERT ON field_availability BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(ProjectService::apply_template_fields(
        &f.state,
        f.project,
        before.global_revision,
        FieldCommand::Create {
            name: "Mass".into(),
            field_kind: FieldKind::Number,
            unit: Some("tons".into()),
            provider: f.provider(),
            options: vec![],
            value: None
        }
    )
    .is_err());
    assert_eq!(f.catalog(), before);
}

#[test]
fn stale_template_update_cannot_overwrite_other_work() {
    let f = Fixture::new();
    let old = f.catalog();
    let id = f.field(Some("years"));
    let before = f.catalog();
    assert!(ProjectService::apply_template_fields(
        &f.state,
        f.project,
        old.global_revision,
        FieldCommand::Unbind {
            field_id: id,
            provider: f.provider()
        }
    )
    .is_err());
    assert_eq!(f.catalog(), before);
}

#[test]
fn units_and_templates_survive_reopen_and_restore_as_copy() {
    let f = Fixture::new();
    let id = f.field(Some("gold crowns"));
    let entry =
        ProjectService::create_entry(&f.state, f.project, Some(f.category), None, None).unwrap();
    let before = ProjectService::apply_fields(
        &f.state,
        f.project,
        entry.id,
        f.catalog().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(FieldValue::Number(25.5)),
            }],
        },
    )
    .unwrap();
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(
        ProjectService::read_fields(&f.state, f.project, entry.id).unwrap(),
        before
    );
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        &backup,
        &f.dir.path().join("copies"),
        None,
    )
    .unwrap();
    assert_ne!(copy.project_id, f.project);
    assert_eq!(
        ProjectService::read_fields(&f.state, copy.project_id, entry.id).unwrap(),
        before
    );
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}

#[test]
fn schema_four_upgrade_preserves_plain_numbers_and_is_retryable() {
    let f = Fixture::new();
    let id = f.field(None);
    let entry =
        ProjectService::create_entry(&f.state, f.project, Some(f.category), None, None).unwrap();
    ProjectService::apply_fields(
        &f.state,
        f.project,
        entry.id,
        f.catalog().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(FieldValue::Number(7.0)),
            }],
        },
    )
    .unwrap();
    ProjectService::close_project(&f.state, f.project).unwrap();
    let conn = f.db();
    conn.execute_batch("DROP TRIGGER search_source_updated; DROP TRIGGER search_source_created; DROP TABLE search_index; DROP TABLE derived_index_state; DROP TABLE entry_alias; DROP TABLE story_link_role; DROP TABLE story_link; DROP TABLE story_role; DROP TABLE rich_document; DROP TABLE story_unit; DROP TRIGGER entry_materialize_capabilities; DROP TABLE spatial_node; DROP TABLE entry_capability; DROP TABLE category_capability_default; DROP TABLE type_capability_default; DROP TABLE capability_def; DROP TABLE field_projection; DROP TRIGGER projection_value_insert; DROP TRIGGER projection_value_update; DROP TABLE entry_field_presentation; DROP TRIGGER field_unit_preserve_values; ALTER TABLE field_definition DROP COLUMN unit; PRAGMA user_version=4; UPDATE project_meta SET schema_version=4; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected upgrade failure'); END;").unwrap();
    let manifest_path = std::path::Path::new(&f.path).join("manifest.json");
    let mut manifest = Manifest::read(&manifest_path).unwrap();
    manifest.schema_version = 4;
    manifest.write(&manifest_path).unwrap();
    assert!(ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).is_err());
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert!(conn.prepare("SELECT unit FROM field_definition").is_err());
    conn.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(conn);
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    let fields = ProjectService::read_fields(&f.state, f.project, entry.id).unwrap();
    assert_eq!(fields.fields[0].value, Some(FieldValue::Number(7.0)));
    assert_eq!(fields.fields[0].definition.unit, None);
    let recovery = f
        .dir
        .path()
        .join(".worldcrafter-migration-recovery")
        .join(f.project.to_string())
        .join("schema-v4.sqlite");
    let db =
        Connection::open_with_flags(recovery, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT number_value FROM field_value WHERE field_id=?1",
            params![id.to_string()],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        7.0
    );
}
