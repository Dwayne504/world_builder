use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{fields::*, structure::FieldId, Entry, ProjectId},
    package::Manifest,
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
        let result = self.apply(FieldCommand::Create {
            name: "A field".into(),
            field_kind: kind,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: self.entry.id.to_string(),
            },
            options: vec![],
            value,
        });
        result.definitions.last().unwrap().id
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

#[test]
fn typed_optional_values_round_trip_and_backup_restore_preserves_internal_ids() {
    let f = Fixture::new();
    for (kind, value) in [
        (FieldKind::ShortText, FieldValue::Text("Unknown".into())),
        (FieldKind::Number, FieldValue::Number(0.0)),
        (FieldKind::Boolean, FieldValue::Boolean(false)),
    ] {
        let id = f.create(kind, None);
        let empty = f.read();
        assert!(empty
            .fields
            .iter()
            .find(|v| v.definition.id == id)
            .unwrap()
            .value
            .is_none());
        f.set(id, Some(value.clone()));
        assert_eq!(
            f.read()
                .fields
                .iter()
                .find(|v| v.definition.id == id)
                .unwrap()
                .value,
            Some(value)
        );
    }
    let before = f.read();
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(f.read(), before);
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        &backup,
        &f.dir.path().join("restored"),
        None,
    )
    .unwrap();
    assert_ne!(copy.project_id, f.project);
    let restored = ProjectService::read_fields(&f.state, copy.project_id, f.entry.id).unwrap();
    assert_eq!(restored.fields, before.fields);
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}

#[test]
fn promotion_and_recursive_type_availability_preserve_definition_and_values() {
    let f = Fixture::new();
    let parent =
        ProjectService::create_type(&f.state, f.project, f.entry.category_id, None, "Parent")
            .unwrap();
    let child = ProjectService::create_type(
        &f.state,
        f.project,
        f.entry.category_id,
        Some(parent.id),
        "Child",
    )
    .unwrap();
    let other = ProjectService::create_entry(
        &f.state,
        f.project,
        Some(f.entry.category_id),
        Some(child.id),
        None,
    )
    .unwrap();
    let id = f.create(FieldKind::Number, Some(FieldValue::Number(92.0)));
    f.apply(FieldCommand::Bind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Type,
            id: parent.id.to_string(),
        },
    });
    let inherited = ProjectService::read_fields(&f.state, f.project, other.id).unwrap();
    assert_eq!(inherited.fields.len(), 1);
    assert_eq!(inherited.fields[0].definition.id, id);
    assert_eq!(inherited.fields[0].value, None);
    assert_eq!(f.read().fields[0].value, Some(FieldValue::Number(92.0)));
    f.apply(FieldCommand::Unbind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Type,
            id: parent.id.to_string(),
        },
    });
    assert!(ProjectService::read_fields(&f.state, f.project, other.id)
        .unwrap()
        .fields
        .is_empty());
}

#[test]
fn template_changes_detach_and_retirement_never_erase_authored_values() {
    let f = Fixture::new();
    let id = f.create(FieldKind::ShortText, Some(FieldValue::Text("green".into())));
    f.apply(FieldCommand::Bind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Category,
            id: f.entry.category_id.to_string(),
        },
    });
    f.apply(FieldCommand::Unbind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Entry,
            id: f.entry.id.to_string(),
        },
    });
    let category = ProjectService::create_category(&f.state, f.project, "Places").unwrap();
    ProjectService::change_entry_structure(
        &f.state,
        f.project,
        f.entry.id,
        f.entry.revision,
        category.id,
        None,
    )
    .unwrap();
    assert!(!f.read().fields[0].available);
    f.apply(FieldCommand::Rename {
        field_id: id,
        name: "Eye colour".into(),
    });
    f.apply(FieldCommand::SetRetired {
        field_id: id,
        retired: true,
    });
    assert_eq!(
        f.read().fields[0].value,
        Some(FieldValue::Text("green".into()))
    );
    f.apply(FieldCommand::SetRetired {
        field_id: id,
        retired: false,
    });
    assert_eq!(f.read().fields[0].definition.id, id);
    f.set(id, None);
    assert!(f.read().fields.is_empty());
}

#[test]
fn choice_options_have_stable_ids_and_retired_selections_are_preserved() {
    let f = Fixture::new();
    let result = f.apply(FieldCommand::Create {
        name: "Eye colour".into(),
        field_kind: FieldKind::MultiChoice,
        provider: FieldProvider {
            kind: ProviderKind::Category,
            id: f.entry.category_id.to_string(),
        },
        options: vec!["Green".into(), "Blue".into()],
        value: None,
    });
    let d = &result.definitions[0];
    let green = d.options[0].id;
    let blue = d.options[1].id;
    f.set(d.id, Some(FieldValue::Choices(vec![green])));
    f.apply(FieldCommand::RenameChoice {
        option_id: green,
        label: "Emerald".into(),
    });
    f.apply(FieldCommand::SetChoiceRetired {
        option_id: green,
        retired: true,
    });
    f.set(d.id, Some(FieldValue::Choices(vec![green, blue])));
    let after = f.read();
    assert_eq!(after.fields[0].definition.options[0].label, "Emerald");
    assert!(after.fields[0].definition.options[0].retired);
    let other = ProjectService::create_entry(&f.state, f.project, None, None, None).unwrap();
    let revision = f.read().global_revision;
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        other.id,
        revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: d.id,
                value: Some(FieldValue::Choices(vec![green]))
            }]
        }
    )
    .is_err());
    f.apply(FieldCommand::SetChoiceRetired {
        option_id: green,
        retired: false,
    });
    assert_eq!(f.read().fields[0].definition.options[0].id, green);
}

#[test]
fn stale_writes_and_partial_invalid_batches_leave_every_value_and_revision_unchanged() {
    let f = Fixture::new();
    let id = f.create(FieldKind::Number, Some(FieldValue::Number(43.0)));
    let before = f.read();
    let command = FieldCommand::SetValues {
        edits: vec![FieldEdit {
            field_id: id,
            value: Some(FieldValue::Number(44.0)),
        }],
    };
    let error = ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision - 1,
        command,
    )
    .unwrap_err();
    assert_eq!(error.kind(), "revision_conflict");
    assert_eq!(before, f.read());
    let command = FieldCommand::SetValues {
        edits: vec![
            FieldEdit {
                field_id: id,
                value: Some(FieldValue::Number(44.0)),
            },
            FieldEdit {
                field_id: FieldId::new(),
                value: None,
            },
        ],
    };
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        command
    )
    .is_err());
    assert_eq!(before, f.read());
    for value in [
        FieldValue::Text("Unknown".into()),
        FieldValue::Number(f64::INFINITY),
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
        assert_eq!(before, f.read());
    }
}

#[test]
fn choice_cardinality_identity_and_empty_storage_are_enforced() {
    let f = Fixture::new();
    let mut definitions = Vec::new();
    for _ in 0..2 {
        let snapshot = f.apply(FieldCommand::Create {
            name: "Same name".into(),
            field_kind: FieldKind::Choice,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: f.entry.id.to_string(),
            },
            options: vec!["One".into(), "Two".into()],
            value: None,
        });
        definitions.push(snapshot.definitions.last().unwrap().clone());
    }
    assert_ne!(definitions[0].id, definitions[1].id);
    let conn = Connection::open_with_flags(
        std::path::Path::new(&f.path).join("data/project.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM field_value", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let d = &definitions[0];
    let before = f.read();
    for ids in [
        vec![d.options[0].id, d.options[1].id],
        vec![definitions[1].options[0].id],
        vec![d.options[0].id, d.options[0].id],
    ] {
        assert!(ProjectService::apply_fields(
            &f.state,
            f.project,
            f.entry.id,
            before.global_revision,
            FieldCommand::SetValues {
                edits: vec![FieldEdit {
                    field_id: d.id,
                    value: Some(FieldValue::Choices(ids))
                }]
            }
        )
        .is_err());
        assert_eq!(before, f.read());
    }
    f.set(d.id, Some(FieldValue::Choices(vec![d.options[0].id])));
    let value_id: String = conn
        .query_row("SELECT id FROM field_value", [], |r| r.get(0))
        .unwrap();
    f.set(d.id, Some(FieldValue::Choices(vec![d.options[1].id])));
    assert_eq!(
        conn.query_row("SELECT id FROM field_value", [], |r| r.get::<_, String>(0))
            .unwrap(),
        value_id
    );
    f.set(d.id, None);
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM field_value", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn nonexistent_providers_and_cross_project_fields_cannot_be_written() {
    let f = Fixture::new();
    let other = Fixture::new();
    let foreign = other.create(FieldKind::Boolean, Some(FieldValue::Boolean(true)));
    let before = f.read();
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        FieldCommand::Bind {
            field_id: foreign,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: f.entry.id.to_string()
            }
        }
    )
    .is_err());
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        FieldCommand::Create {
            name: "Invalid".into(),
            field_kind: FieldKind::Boolean,
            provider: FieldProvider {
                kind: ProviderKind::Category,
                id: "missing".into()
            },
            options: vec![],
            value: None
        }
    )
    .is_err());
    assert_eq!(before, f.read());
}

#[test]
fn schema_two_migrates_with_a_valid_external_recovery_point_and_preserves_entries() {
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let paths =
        worldcrafter_lib::package::layout::create_skeleton(&dir.path().join("v2.wcproj")).unwrap();
    let project = ProjectId::new();
    let entry = worldcrafter_lib::domain::EntryId::new();
    let category = worldcrafter_lib::domain::CategoryId::new();
    Manifest::new(project, 1, 2, "Old Project")
        .write(&paths.manifest_path())
        .unwrap();
    let conn = Connection::open(paths.db_path()).unwrap();
    conn.execute_batch(include_str!("../src/persistence/migrations/0001_init.sql"))
        .unwrap();
    conn.execute_batch(include_str!(
        "../src/persistence/migrations/0002_project_structure.sql"
    ))
    .unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("INSERT INTO project_meta(id,project_id,format_version,schema_version,working_name,created_at,updated_at) VALUES(1,?1,1,2,'Old Project',?2,?2)",params![project.to_string(),now]).unwrap();
    conn.execute(
        "INSERT INTO category VALUES(?1,'Uncategorized',1,?2,?2,0)",
        params![category.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO record_identity VALUES(?1,'entry','active',?2,?2)",
        params![entry.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO entry VALUES(?1,?2,NULL,'Legacy Entry',?3,?3,1)",
        params![entry.to_string(), category.to_string(), now],
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 2).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER interrupt_fields_migration BEFORE UPDATE OF schema_version ON project_meta
        BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;",
    )
    .unwrap();
    drop(conn);
    assert!(ProjectService::open_project(&state, &paths.root, false).is_err());
    let conn = Connection::open(paths.db_path()).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'field_definition'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        Manifest::read(&paths.manifest_path())
            .unwrap()
            .schema_version,
        2
    );
    conn.execute_batch("DROP TRIGGER interrupt_fields_migration")
        .unwrap();
    drop(conn);
    let opened = ProjectService::open_project(&state, &paths.root, false).unwrap();
    assert_eq!(
        opened.schema_version,
        worldcrafter_lib::persistence::migrations::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(
        ProjectService::get_entry(&state, project, entry)
            .unwrap()
            .authored_name
            .as_deref(),
        Some("Legacy Entry")
    );
    assert!(ProjectService::read_fields(&state, project, entry)
        .unwrap()
        .fields
        .is_empty());
    let recovery = dir
        .path()
        .join(".worldcrafter-migration-recovery")
        .join(project.to_string())
        .join("schema-v2.sqlite");
    let recovery =
        Connection::open_with_flags(recovery, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    ProjectService::close_project(&state, project).unwrap();
    assert_eq!(
        Manifest::read(&paths.manifest_path())
            .unwrap()
            .schema_version,
        worldcrafter_lib::persistence::migrations::CURRENT_SCHEMA_VERSION
    );
}
