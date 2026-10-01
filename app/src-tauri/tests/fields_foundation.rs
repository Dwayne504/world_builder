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
        unit: None,
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
            unit: None,
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
            unit: None,
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

fn duplicate_number(f: &Fixture, unit: Option<&str>, value: Option<f64>) -> FieldId {
    let before = f.read();
    let result = f.apply(FieldCommand::Create {
        name: "Age".into(),
        field_kind: FieldKind::Number,
        unit: unit.map(str::to_owned),
        provider: FieldProvider {
            kind: ProviderKind::Entry,
            id: f.entry.id.to_string(),
        },
        options: vec![],
        value: value.map(FieldValue::Number),
    });
    result
        .definitions
        .iter()
        .find(|d| !before.definitions.iter().any(|b| b.id == d.id))
        .unwrap()
        .id
}

#[test]
fn reviewed_merge_preserves_value_identity_bindings_and_restorable_snapshot() {
    let f = Fixture::new();
    let source = duplicate_number(&f, Some("years"), Some(48.0));
    let target = duplicate_number(&f, Some("years"), None);
    f.apply(FieldCommand::Bind {
        field_id: target,
        provider: FieldProvider {
            kind: ProviderKind::Category,
            id: f.entry.category_id.to_string(),
        },
    });
    let paths = worldcrafter_lib::package::PackagePaths::new(&f.path);
    let db = Connection::open(paths.db_path()).unwrap();
    let value_id: String = db
        .query_row(
            "SELECT id FROM field_value WHERE field_id=?1",
            [source.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    let preview = ProjectService::preview_field_merge(&f.state, f.project, source, target).unwrap();
    assert!(preview.blockers.is_empty());
    assert_eq!(preview.entries.len(), 1);
    assert_eq!(
        preview.entries[0].source_value,
        Some(FieldValue::Number(48.0))
    );
    let result = ProjectService::merge_fields(
        &f.state,
        f.project,
        source,
        target,
        preview.global_revision,
        &f.dir.path().join("Backups"),
    )
    .unwrap();
    assert_eq!(result.global_revision, preview.global_revision + 1);
    let after = f.read();
    assert_eq!(after.definitions.len(), 1);
    assert_eq!(after.definitions[0].id, target);
    assert_eq!(after.definitions[0].bindings.len(), 2);
    assert_eq!(after.fields[0].value, Some(FieldValue::Number(48.0)));
    assert_eq!(
        db.query_row(
            "SELECT id FROM field_value WHERE field_id=?1",
            [target.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        value_id
    );
    drop(db);
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        std::path::Path::new(&result.backup_path),
        &f.dir.path().join("Restored"),
        Some("Before merge"),
    )
    .unwrap();
    let restored = ProjectService::read_fields(&f.state, copy.project_id, f.entry.id).unwrap();
    assert_eq!(restored.definitions.len(), 2);
    assert_eq!(
        restored
            .fields
            .iter()
            .find(|v| v.definition.id == source)
            .unwrap()
            .value,
        Some(FieldValue::Number(48.0))
    );
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}

#[test]
fn merge_blocks_conflicting_values_units_kinds_and_cross_project_ids_without_writing() {
    let f = Fixture::new();
    let source = duplicate_number(&f, Some("years"), Some(48.0));
    let target = duplicate_number(&f, Some("years"), Some(49.0));
    let before = f.read();
    let backup = f.dir.path().join("Backups");
    let preview = ProjectService::preview_field_merge(&f.state, f.project, source, target).unwrap();
    assert!(preview.entries[0].conflict);
    assert!(ProjectService::merge_fields(
        &f.state,
        f.project,
        source,
        target,
        preview.global_revision,
        &backup
    )
    .is_err());
    assert_eq!(f.read(), before);
    assert!(!backup.exists());
    let months = duplicate_number(&f, Some("months"), None);
    assert!(
        !ProjectService::preview_field_merge(&f.state, f.project, source, months)
            .unwrap()
            .blockers
            .is_empty()
    );
    let text = f.create(FieldKind::ShortText, None);
    assert!(
        !ProjectService::preview_field_merge(&f.state, f.project, source, text)
            .unwrap()
            .blockers
            .is_empty()
    );
    assert!(
        ProjectService::preview_field_merge(&f.state, f.project, source, FieldId::new()).is_err()
    );
    assert!(ProjectService::preview_field_merge(&f.state, f.project, source, source).is_err());
}

#[test]
fn merge_equal_values_and_stale_reviews_are_checked_before_backup() {
    let f = Fixture::new();
    let source = duplicate_number(&f, None, Some(48.0));
    let target = duplicate_number(&f, None, Some(48.0));
    let preview = ProjectService::preview_field_merge(&f.state, f.project, source, target).unwrap();
    f.set(source, Some(FieldValue::Number(50.0)));
    let backup = f.dir.path().join("Backups");
    assert!(ProjectService::merge_fields(
        &f.state,
        f.project,
        source,
        target,
        preview.global_revision,
        &backup
    )
    .is_err());
    assert!(!backup.exists());
    f.set(source, Some(FieldValue::Number(48.0)));
    let result = ProjectService::merge_fields(
        &f.state,
        f.project,
        source,
        target,
        f.read().global_revision,
        &backup,
    )
    .unwrap();
    assert!(std::path::Path::new(&result.backup_path).is_dir());
    assert_eq!(f.read().fields.len(), 1);
    assert_eq!(f.read().fields[0].value, Some(FieldValue::Number(48.0)));
}

#[test]
fn backup_failure_and_transaction_failure_leave_merge_sources_untouched() {
    let f = Fixture::new();
    let source = duplicate_number(&f, None, Some(48.0));
    let target = duplicate_number(&f, None, None);
    let before = f.read();
    let backup_file = f.dir.path().join("not-a-directory");
    std::fs::write(&backup_file, b"preserve").unwrap();
    assert!(ProjectService::merge_fields(
        &f.state,
        f.project,
        source,
        target,
        before.global_revision,
        &backup_file
    )
    .is_err());
    assert_eq!(f.read(), before);
    let db =
        Connection::open(worldcrafter_lib::package::PackagePaths::new(&f.path).db_path()).unwrap();
    db.execute_batch("CREATE TRIGGER injected_merge_failure BEFORE DELETE ON field_definition BEGIN SELECT RAISE(ABORT, 'injected merge failure'); END;").unwrap();
    assert!(ProjectService::merge_fields(
        &f.state,
        f.project,
        source,
        target,
        before.global_revision,
        &f.dir.path().join("Backups")
    )
    .is_err());
    assert_eq!(f.read(), before);
}

#[test]
fn concurrent_merge_requests_commit_once_and_preserve_other_entries() {
    let f = Fixture::new();
    let source = duplicate_number(&f, Some("years"), Some(48.0));
    let target = duplicate_number(&f, Some("years"), None);
    f.apply(FieldCommand::Bind {
        field_id: source,
        provider: FieldProvider {
            kind: ProviderKind::Category,
            id: f.entry.category_id.to_string(),
        },
    });
    let other = ProjectService::create_entry(
        &f.state,
        f.project,
        Some(f.entry.category_id),
        None,
        Some("Second".into()),
    )
    .unwrap();
    ProjectService::apply_fields(
        &f.state,
        f.project,
        other.id,
        f.read().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: source,
                value: Some(FieldValue::Number(17.0)),
            }],
        },
    )
    .unwrap();
    let expected = f.read().global_revision;
    let backups = f.dir.path().join("Backups");
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            ProjectService::merge_fields(&f.state, f.project, source, target, expected, &backups)
        });
        let b = scope.spawn(|| {
            ProjectService::merge_fields(&f.state, f.project, source, target, expected, &backups)
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        std::fs::read_dir(backups.join(f.project.to_string()))
            .unwrap()
            .count(),
        1
    );
    let after = ProjectService::read_fields(&f.state, f.project, other.id).unwrap();
    assert_eq!(after.fields.len(), 1);
    assert_eq!(after.fields[0].definition.id, target);
    assert_eq!(after.fields[0].value, Some(FieldValue::Number(17.0)));
    assert_eq!(after.global_revision, expected + 1);
}

#[test]
fn scalar_merges_keep_false_and_text_while_choice_and_retired_definitions_block() {
    for (kind, value) in [
        (FieldKind::Boolean, FieldValue::Boolean(false)),
        (
            FieldKind::ShortText,
            FieldValue::Text("Authored words".into()),
        ),
    ] {
        let f = Fixture::new();
        let source = f.create(kind, Some(value.clone()));
        let target = f.create(kind, None);
        ProjectService::merge_fields(
            &f.state,
            f.project,
            source,
            target,
            f.read().global_revision,
            &f.dir.path().join("Backups"),
        )
        .unwrap();
        assert_eq!(f.read().fields[0].value, Some(value));
    }
    let f = Fixture::new();
    let source = f.create(FieldKind::Choice, None);
    let target = f.create(FieldKind::Choice, None);
    assert!(
        !ProjectService::preview_field_merge(&f.state, f.project, source, target)
            .unwrap()
            .blockers
            .is_empty()
    );
    let source = duplicate_number(&f, None, Some(48.0));
    let target = duplicate_number(&f, None, None);
    f.apply(FieldCommand::SetRetired {
        field_id: source,
        retired: true,
    });
    assert!(
        !ProjectService::preview_field_merge(&f.state, f.project, source, target)
            .unwrap()
            .blockers
            .is_empty()
    );
}

#[test]
fn hidden_fields_are_local_persisted_and_inherited_defaults_are_identified() {
    let f = Fixture::new();
    let parent =
        ProjectService::create_type(&f.state, f.project, f.entry.category_id, None, "Human")
            .unwrap();
    let child = ProjectService::create_type(
        &f.state,
        f.project,
        f.entry.category_id,
        Some(parent.id),
        "Scholar",
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
    let id = f.create(FieldKind::Number, Some(FieldValue::Number(48.0)));
    f.apply(FieldCommand::Bind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Type,
            id: parent.id.to_string(),
        },
    });
    let inherited = ProjectService::read_fields(&f.state, f.project, other.id).unwrap();
    assert_eq!(inherited.fields[0].default_sources[0].label, "Human");
    assert!(f.read().fields[0].default_sources.is_empty());
    let before = f.read();
    f.apply(FieldCommand::SetHidden {
        field_id: id,
        hidden: true,
    });
    let hidden = f.read();
    assert!(hidden.fields[0].hidden);
    assert_eq!(hidden.fields[0].value, before.fields[0].value);
    assert_eq!(hidden.definitions, before.definitions);
    assert!(
        !ProjectService::read_fields(&f.state, f.project, other.id)
            .unwrap()
            .fields[0]
            .hidden
    );
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(f.read(), hidden);
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("Backups")).unwrap();
    let restored = ProjectService::restore_backup_as_copy(
        &f.state,
        &backup,
        &f.dir.path().join("Copies"),
        None,
    )
    .unwrap();
    assert_eq!(
        ProjectService::read_fields(&f.state, restored.project_id, f.entry.id)
            .unwrap()
            .fields,
        hidden.fields
    );
    ProjectService::close_project(&f.state, restored.project_id).unwrap();
    f.apply(FieldCommand::SetHidden {
        field_id: id,
        hidden: false,
    });
    assert!(!f.read().fields[0].hidden);
    assert_eq!(f.read().fields[0].value, before.fields[0].value);
}

#[test]
fn hiding_rejects_stale_or_unavailable_fields_without_mutation() {
    let f = Fixture::new();
    let id = f.create(FieldKind::ShortText, None);
    let before = f.read();
    for (field_id, revision) in [
        (id, before.global_revision - 1),
        (FieldId::new(), before.global_revision),
    ] {
        assert!(ProjectService::apply_fields(
            &f.state,
            f.project,
            f.entry.id,
            revision,
            FieldCommand::SetHidden {
                field_id,
                hidden: true
            }
        )
        .is_err());
        assert_eq!(f.read(), before);
    }
    f.apply(FieldCommand::Unbind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Entry,
            id: f.entry.id.to_string(),
        },
    });
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        f.read().global_revision,
        FieldCommand::SetHidden {
            field_id: id,
            hidden: true
        }
    )
    .is_err());
}

#[test]
fn delete_is_entry_local_suppresses_defaults_and_readd_is_empty() {
    let f = Fixture::new();
    let id = f.create(FieldKind::Number, Some(FieldValue::Number(48.0)));
    let default = FieldProvider {
        kind: ProviderKind::Category,
        id: f.entry.category_id.to_string(),
    };
    f.apply(FieldCommand::Bind {
        field_id: id,
        provider: default.clone(),
    });
    let other =
        ProjectService::create_entry(&f.state, f.project, Some(f.entry.category_id), None, None)
            .unwrap();
    ProjectService::apply_fields(
        &f.state,
        f.project,
        other.id,
        f.read().global_revision,
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: id,
                value: Some(FieldValue::Number(17.0)),
            }],
        },
    )
    .unwrap();
    f.apply(FieldCommand::SetHidden {
        field_id: id,
        hidden: true,
    });
    let before = f.read();
    let other_before = ProjectService::read_fields(&f.state, f.project, other.id).unwrap();
    let outcome = ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        id,
        before.global_revision,
        &f.dir.path().join("Backups"),
    )
    .unwrap();
    assert!(outcome.snapshot.fields.is_empty());
    assert_eq!(outcome.snapshot.global_revision, before.global_revision + 1);
    assert_eq!(outcome.snapshot.definitions.len(), 1);
    assert_eq!(outcome.snapshot.definitions[0].bindings.len(), 1);
    assert_eq!(
        ProjectService::read_fields(&f.state, f.project, other.id)
            .unwrap()
            .fields[0]
            .value,
        other_before.fields[0].value
    );
    // A shared default change must not silently undo a local removal.
    f.apply(FieldCommand::Unbind {
        field_id: id,
        provider: default.clone(),
    });
    f.apply(FieldCommand::Bind {
        field_id: id,
        provider: default,
    });
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert!(f.read().fields.is_empty());
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        std::path::Path::new(&outcome.backup_path),
        &f.dir.path().join("Copies"),
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
    f.apply(FieldCommand::Bind {
        field_id: id,
        provider: FieldProvider {
            kind: ProviderKind::Entry,
            id: f.entry.id.to_string(),
        },
    });
    assert_eq!(f.read().fields[0].value, None);
    assert!(!f.read().fields[0].hidden);
    assert_eq!(f.read().fields[0].default_sources.len(), 1);
}

#[test]
fn delete_cleans_choice_selections_without_removing_options_or_other_values() {
    let f = Fixture::new();
    let created = f.apply(FieldCommand::Create {
        name: "Colours".into(),
        field_kind: FieldKind::MultiChoice,
        unit: None,
        provider: FieldProvider {
            kind: ProviderKind::Category,
            id: f.entry.category_id.to_string(),
        },
        options: vec!["Blue".into(), "Green".into()],
        value: None,
    });
    let id = created.definitions[0].id;
    let choices = created.definitions[0]
        .options
        .iter()
        .map(|o| o.id)
        .collect();
    f.set(id, Some(FieldValue::Choices(choices)));
    let outcome = ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        id,
        f.read().global_revision,
        &f.dir.path().join("Backups"),
    )
    .unwrap();
    assert!(outcome.snapshot.fields.is_empty());
    assert_eq!(outcome.snapshot.definitions[0].options.len(), 2);
    let db =
        Connection::open(worldcrafter_lib::package::PackagePaths::new(&f.path).db_path()).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM field_choice_value", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}

#[test]
fn delete_rejects_stale_review_bad_backup_and_rolls_back_transaction_failure() {
    let f = Fixture::new();
    let id = f.create(FieldKind::Number, Some(FieldValue::Number(48.0)));
    let before = f.read();
    let backups = f.dir.path().join("Backups");
    assert!(ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        id,
        before.global_revision - 1,
        &backups
    )
    .is_err());
    assert!(ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        FieldId::new(),
        before.global_revision,
        &backups
    )
    .is_err());
    assert!(!backups.exists());
    let file = f.dir.path().join("not-a-directory");
    std::fs::write(&file, b"preserve").unwrap();
    assert!(ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        id,
        before.global_revision,
        &file
    )
    .is_err());
    assert_eq!(f.read(), before);
    let db =
        Connection::open(worldcrafter_lib::package::PackagePaths::new(&f.path).db_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_local_delete BEFORE INSERT ON entry_field_presentation BEGIN SELECT RAISE(ABORT, 'injected deletion failure'); END;").unwrap();
    assert!(ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        id,
        before.global_revision,
        &backups
    )
    .is_err());
    assert_eq!(f.read(), before);
    assert_eq!(
        std::fs::read_dir(backups.join(f.project.to_string()))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn concurrent_entry_deletions_commit_and_back_up_once() {
    let f = Fixture::new();
    let id = f.create(FieldKind::Boolean, Some(FieldValue::Boolean(false)));
    let revision = f.read().global_revision;
    let backups = f.dir.path().join("Backups");
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            ProjectService::delete_entry_field(
                &f.state, f.project, f.entry.id, id, revision, &backups,
            )
        });
        let b = scope.spawn(|| {
            ProjectService::delete_entry_field(
                &f.state, f.project, f.entry.id, id, revision, &backups,
            )
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(f.read().global_revision, revision + 1);
    assert_eq!(
        std::fs::read_dir(backups.join(f.project.to_string()))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn duplicate_merge_carries_local_visibility_and_respects_existing_target_choice() {
    for target_choice in [None, Some(false), Some(true)] {
        let f = Fixture::new();
        let source = duplicate_number(&f, Some("years"), Some(48.0));
        let target = duplicate_number(&f, Some("years"), None);
        f.apply(FieldCommand::SetHidden {
            field_id: source,
            hidden: true,
        });
        if let Some(hidden) = target_choice {
            f.apply(FieldCommand::SetHidden {
                field_id: target,
                hidden,
            });
        }
        ProjectService::merge_fields(
            &f.state,
            f.project,
            source,
            target,
            f.read().global_revision,
            &f.dir.path().join("Backups"),
        )
        .unwrap();
        let after = f.read();
        assert_eq!(after.fields.len(), 1);
        assert_eq!(after.fields[0].value, Some(FieldValue::Number(48.0)));
        assert_eq!(after.fields[0].hidden, target_choice.unwrap_or(true));
    }
}

#[test]
fn version_five_upgrade_is_recoverable_and_does_not_rewrite_values() {
    let f = Fixture::new();
    f.create(FieldKind::Number, Some(FieldValue::Number(48.0)));
    let before = f.read();
    ProjectService::close_project(&f.state, f.project).unwrap();
    let paths = worldcrafter_lib::package::PackagePaths::new(&f.path);
    let db = Connection::open(paths.db_path()).unwrap();
    db.execute_batch("DROP TRIGGER search_source_updated; DROP TRIGGER search_source_created; DROP TABLE search_index; DROP TABLE derived_index_state; DROP TABLE entry_alias; DROP TABLE story_link_role; DROP TABLE story_link; DROP TABLE story_role; DROP TABLE rich_document; DROP TABLE story_unit; DROP TRIGGER entry_materialize_capabilities; DROP TABLE spatial_node; DROP TABLE entry_capability; DROP TABLE category_capability_default; DROP TABLE type_capability_default; DROP TABLE capability_def; DROP TABLE field_projection; DROP TRIGGER projection_value_insert; DROP TRIGGER projection_value_update; DROP TABLE entry_field_presentation; PRAGMA user_version=5; UPDATE project_meta SET schema_version=5; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT, 'injected upgrade failure'); END;").unwrap();
    let mut manifest = Manifest::read(&paths.manifest_path()).unwrap();
    manifest.schema_version = 5;
    manifest.write(&paths.manifest_path()).unwrap();
    assert!(ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='entry_field_presentation'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER fail_upgrade;").unwrap();
    drop(db);
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(f.read().fields, before.fields);
    assert!(f
        .dir
        .path()
        .join(".worldcrafter-migration-recovery")
        .exists());
}

#[test]
fn merge_reviews_removed_fields_and_preserves_the_kept_local_removal() {
    let f = Fixture::new();
    let source = duplicate_number(&f, None, None);
    let target = duplicate_number(&f, None, None);
    let backups = f.dir.path().join("Backups");
    for field in [source, target] {
        ProjectService::delete_entry_field(
            &f.state,
            f.project,
            f.entry.id,
            field,
            f.read().global_revision,
            &backups,
        )
        .unwrap();
    }
    let preview = ProjectService::preview_field_merge(&f.state, f.project, source, target).unwrap();
    assert_eq!(preview.entries.len(), 1);
    assert_eq!(preview.entries[0].entry_id, f.entry.id);
    ProjectService::merge_fields(
        &f.state,
        f.project,
        source,
        target,
        preview.global_revision,
        &backups,
    )
    .unwrap();
    f.apply(FieldCommand::Bind {
        field_id: target,
        provider: FieldProvider {
            kind: ProviderKind::Category,
            id: f.entry.category_id.to_string(),
        },
    });
    assert!(f.read().fields.is_empty());
    assert_eq!(f.read().definitions.len(), 1);
    let db =
        Connection::open(worldcrafter_lib::package::PackagePaths::new(&f.path).db_path()).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM entry_field_presentation WHERE field_id=?1",
            [source.to_string()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(db
        .query_row(
            "SELECT removed FROM entry_field_presentation WHERE field_id=?1",
            [target.to_string()],
            |r| r.get::<_, bool>(0)
        )
        .unwrap());
}
