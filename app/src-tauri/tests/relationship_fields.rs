use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        fields::*,
        relationships::*,
        structure::{FieldId, RelationshipDefinitionId, RelationshipId},
        Entry, EntryId, ProjectId,
    },
};

struct Fixture {
    dir: TempDir,
    state: AppState,
    project: ProjectId,
    entry: Entry,
    path: String,
    ownership: RelationshipDefinitionId,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let p = ProjectService::create_project(&state, dir.path(), "Projection test").unwrap();
        let entry =
            ProjectService::create_entry(&state, p.project_id, None, None, Some("Artifact".into()))
                .unwrap();
        let before = ProjectService::read_relationships(&state, p.project_id, entry.id).unwrap();
        let definitions = ProjectService::apply_relationships(
            &state,
            p.project_id,
            entry.id,
            before.global_revision,
            RelationshipCommand::CreateDefinition {
                draft: DefinitionDraft {
                    name: "Ownership".into(),
                    forward_label: "owns".into(),
                    inverse_label: "is owned by".into(),
                    directed: true,
                    expected_targets_per_source: None,
                    expected_sources_per_target: Some(1),
                },
            },
        )
        .unwrap();
        Self {
            dir,
            state,
            project: p.project_id,
            entry,
            path: p.package_path,
            ownership: definitions.definitions[0].id,
        }
    }
    fn read(&self, entry: EntryId) -> EntryFields {
        ProjectService::read_fields(&self.state, self.project, entry).unwrap()
    }
    fn apply(&self, entry: EntryId, command: FieldCommand) -> EntryFields {
        ProjectService::apply_fields(
            &self.state,
            self.project,
            entry,
            self.read(entry).global_revision,
            command,
        )
        .unwrap()
    }
    fn relation(&self, entry: EntryId, command: RelationshipCommand) -> EntryRelationships {
        ProjectService::apply_relationships(
            &self.state,
            self.project,
            entry,
            self.read(entry).global_revision,
            command,
        )
        .unwrap()
    }
    fn projection(
        &self,
        entry: EntryId,
        definition: RelationshipDefinitionId,
        perspective: Perspective,
    ) -> FieldId {
        self.apply(
            entry,
            FieldCommand::CreateProjection {
                name: "Property".into(),
                relationship_definition_id: definition,
                perspective,
                provider: FieldProvider {
                    kind: ProviderKind::Entry,
                    id: entry.to_string(),
                },
            },
        )
        .fields
        .last()
        .unwrap()
        .definition
        .id
    }
    fn owner_field(&self) -> FieldId {
        self.projection(self.entry.id, self.ownership, Perspective::Target)
    }
    fn new_entry(&self, name: &str) -> EntryId {
        ProjectService::create_entry(&self.state, self.project, None, None, Some(name.into()))
            .unwrap()
            .id
    }
    fn edit(
        &self,
        field_id: FieldId,
        other: EntryId,
        instance_id: Option<RelationshipId>,
    ) -> EntryFields {
        self.apply(
            self.entry.id,
            FieldCommand::EditProjection {
                field_id,
                other: OtherEntry::Existing { id: other },
                instance_id,
            },
        )
    }
    fn connect(&self, other: EntryId, replace: Vec<RelationshipId>) -> EntryRelationships {
        self.relation(
            self.entry.id,
            RelationshipCommand::Connect {
                definition_id: self.ownership,
                perspective: Perspective::Target,
                other: OtherEntry::Existing { id: other },
                note: "Preserved authored note".into(),
                replace,
            },
        )
    }
    fn db(&self) -> Connection {
        let conn =
            Connection::open(std::path::Path::new(&self.path).join("data/project.sqlite")).unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}

#[test]
fn empty_projection_creates_once_and_single_retarget_preserves_identity_note_and_inverse() {
    let f = Fixture::new();
    let field = f.owner_field();
    let first = f.new_entry("First owner");
    let second = f.new_entry("Second owner");
    let owns = f.projection(second, f.ownership, Perspective::Source);
    let created = f.edit(field, first, None).fields[0].projected_relationships[0].clone();
    f.relation(
        f.entry.id,
        RelationshipCommand::SetNote {
            id: created.id,
            note: "Keep this note".into(),
        },
    );
    let changed = f.edit(field, second, None).fields[0].projected_relationships[0].clone();
    assert_eq!(changed.id, created.id);
    assert_eq!(changed.note, "Keep this note");
    assert_eq!(changed.source.id, Some(second));
    assert_eq!(changed.target.id, Some(f.entry.id));
    let inverse = f.read(second);
    assert_eq!(
        inverse
            .fields
            .iter()
            .find(|x| x.definition.id == owns)
            .unwrap()
            .projected_relationships[0]
            .id,
        created.id
    );
    assert!(
        ProjectService::read_relationships(&f.state, f.project, first)
            .unwrap()
            .relationships
            .is_empty()
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM field_value", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(f.read(f.entry.id).fields[0].value.is_none());
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(
        f.read(f.entry.id).fields[0].projected_relationships[0],
        changed
    );
}

#[test]
fn ambiguity_preserves_both_and_requires_selection_or_explicit_replace() {
    let f = Fixture::new();
    let field = f.owner_field();
    let a = f.new_entry("A");
    let b = f.new_entry("B");
    let c = f.new_entry("C");
    let first = f.connect(a, vec![]).relationships[0].id;
    f.connect(b, vec![]);
    let before = f.read(f.entry.id);
    assert_eq!(before.fields[0].projected_relationships.len(), 2);
    assert!(!before.fields[0].projected_relationships[0]
        .warnings
        .is_empty());
    let error = ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        FieldCommand::EditProjection {
            field_id: field,
            other: OtherEntry::Existing { id: c },
            instance_id: None,
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("Multiple current relationships"));
    assert_eq!(f.read(f.entry.id), before);
    let changed = f.edit(field, c, Some(first));
    assert_eq!(changed.fields[0].projected_relationships.len(), 2);
    assert_eq!(
        changed.fields[0]
            .projected_relationships
            .iter()
            .find(|r| r.id == first)
            .unwrap()
            .source
            .id,
        Some(c)
    );
    let ids = changed.fields[0]
        .projected_relationships
        .iter()
        .map(|r| r.id)
        .collect();
    let replaced = f.connect(a, ids);
    assert_eq!(replaced.relationships.iter().filter(|r| r.ended).count(), 2);
    assert_eq!(
        f.read(f.entry.id).fields[0].projected_relationships.len(),
        1
    );
    assert!(replaced
        .relationships
        .iter()
        .filter(|r| r.ended)
        .all(|r| r.note == "Preserved authored note"));
}

#[test]
fn duplicate_retarget_stale_write_and_injected_failure_roll_back_every_participant() {
    let f = Fixture::new();
    let field = f.owner_field();
    let a = f.new_entry("A");
    let b = f.new_entry("B");
    let first = f.connect(a, vec![]).relationships[0].id;
    f.connect(b, vec![]);
    let before = f.read(f.entry.id);
    for (revision, other) in [(before.global_revision, b), (before.global_revision - 1, a)] {
        assert!(ProjectService::apply_fields(
            &f.state,
            f.project,
            f.entry.id,
            revision,
            FieldCommand::EditProjection {
                field_id: field,
                other: OtherEntry::Existing { id: other },
                instance_id: Some(first)
            }
        )
        .is_err());
        assert_eq!(f.read(f.entry.id), before);
    }
    f.db().execute_batch("CREATE TRIGGER fail_projection_retarget BEFORE UPDATE ON relationship_participant WHEN NEW.slot='target' BEGIN SELECT RAISE(ABORT,'injected retarget failure'); END;").unwrap();
    let entries = ProjectService::list_entries(&f.state, f.project)
        .unwrap()
        .len();
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        FieldCommand::EditProjection {
            field_id: field,
            other: OtherEntry::Create {
                name: Some("Rolled back stub".into()),
                category_id: None
            },
            instance_id: Some(first)
        }
    )
    .is_err());
    assert_eq!(f.read(f.entry.id), before);
    assert_eq!(
        ProjectService::list_entries(&f.state, f.project)
            .unwrap()
            .len(),
        entries
    );
}

#[test]
fn quick_create_is_atomic_and_field_data_survives_rename() {
    let f = Fixture::new();
    let field = f.owner_field();
    let snapshot = f.apply(
        f.entry.id,
        FieldCommand::EditProjection {
            field_id: field,
            other: OtherEntry::Create {
                name: Some("New owner".into()),
                category_id: None,
            },
            instance_id: None,
        },
    );
    let relation = snapshot.fields[0].projected_relationships[0].clone();
    assert_eq!(relation.source.label, "New owner");
    f.apply(
        f.entry.id,
        FieldCommand::Rename {
            field_id: field,
            name: "Current keeper".into(),
        },
    );
    let snapshot = f.read(f.entry.id);
    assert_eq!(snapshot.fields[0].definition.name, "Current keeper");
    assert_eq!(snapshot.fields[0].projected_relationships[0], relation);
}

#[test]
fn current_projection_keeps_inactive_targets_but_excludes_ended_or_inactive_instances() {
    let f = Fixture::new();
    let field = f.owner_field();
    let owner = f.new_entry("Owner");
    let id = f.edit(field, owner, None).fields[0].projected_relationships[0].id;
    for state in ["archived", "trashed"] {
        f.db()
            .execute(
                "UPDATE record_identity SET workspace_state=?1 WHERE record_id=?2",
                params![state, owner.to_string()],
            )
            .unwrap();
        let read = f.read(f.entry.id);
        assert_eq!(
            read.fields[0].projected_relationships[0]
                .source
                .workspace_state,
            state
        );
    }
    f.relation(
        f.entry.id,
        RelationshipCommand::SetEnded { id, ended: true },
    );
    assert!(f.read(f.entry.id).fields[0]
        .projected_relationships
        .is_empty());
    f.db()
        .execute(
            "UPDATE record_identity SET workspace_state='active' WHERE record_id=?1",
            [owner.to_string()],
        )
        .unwrap();
    f.relation(
        f.entry.id,
        RelationshipCommand::SetEnded { id, ended: false },
    );
    f.db()
        .execute(
            "UPDATE record_identity SET workspace_state='archived' WHERE record_id=?1",
            [id.to_string()],
        )
        .unwrap();
    assert!(f.read(f.entry.id).fields[0]
        .projected_relationships
        .is_empty());
}

#[test]
fn symmetric_projection_resolves_both_sides_and_self_only_once() {
    let f = Fixture::new();
    let snapshot = f.relation(
        f.entry.id,
        RelationshipCommand::CreateDefinition {
            draft: DefinitionDraft {
                name: "Ally".into(),
                forward_label: "allied with".into(),
                inverse_label: "allied with".into(),
                directed: false,
                expected_targets_per_source: None,
                expected_sources_per_target: None,
            },
        },
    );
    let definition = snapshot
        .definitions
        .iter()
        .find(|d| !d.draft.directed)
        .unwrap()
        .id;
    let field = f.projection(f.entry.id, definition, Perspective::Target);
    let other = f.new_entry("Ally");
    f.projection(other, definition, Perspective::Source);
    let first = f.edit(field, other, None).fields[0].projected_relationships[0].clone();
    assert_eq!(
        f.read(other).fields[0].projected_relationships[0].id,
        first.id
    );
    let self_link = f.edit(field, f.entry.id, None);
    assert_eq!(self_link.fields[0].projected_relationships.len(), 1);
    assert_eq!(
        self_link.fields[0].projected_relationships[0].source.id,
        Some(f.entry.id)
    );
    assert_eq!(
        self_link.fields[0].projected_relationships[0].target.id,
        Some(f.entry.id)
    );
    assert!(f.read(other).fields[0].projected_relationships.is_empty());
}

#[test]
fn presentation_changes_never_end_or_delete_canonical_connections() {
    let f = Fixture::new();
    let field = f.owner_field();
    let owner = f.new_entry("Owner");
    let connection = f.edit(field, owner, None).fields[0].projected_relationships[0].clone();
    let hidden = f.apply(
        f.entry.id,
        FieldCommand::SetHidden {
            field_id: field,
            hidden: true,
        },
    );
    assert!(hidden.fields[0].hidden);
    let retired = f.apply(
        f.entry.id,
        FieldCommand::SetRetired {
            field_id: field,
            retired: true,
        },
    );
    assert!(!retired.fields[0].available);
    assert_eq!(retired.fields[0].projected_relationships[0], connection);
    f.apply(
        f.entry.id,
        FieldCommand::SetRetired {
            field_id: field,
            retired: false,
        },
    );
    let removed = f.apply(
        f.entry.id,
        FieldCommand::RemoveProjection { field_id: field },
    );
    assert!(removed.fields.is_empty());
    assert_eq!(
        ProjectService::read_relationships(&f.state, f.project, owner)
            .unwrap()
            .relationships[0],
        connection
    );
    let restored = f.apply(
        f.entry.id,
        FieldCommand::Bind {
            field_id: field,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: f.entry.id.to_string(),
            },
        },
    );
    assert_eq!(restored.fields[0].projected_relationships[0], connection);
    f.apply(
        f.entry.id,
        FieldCommand::Unbind {
            field_id: field,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: f.entry.id.to_string(),
            },
        },
    );
    assert!(f.read(f.entry.id).fields.is_empty());
    assert_eq!(
        ProjectService::read_relationships(&f.state, f.project, f.entry.id)
            .unwrap()
            .relationships[0],
        connection
    );
}

#[test]
fn scalar_save_delete_and_merge_cannot_change_projected_relationships() {
    let f = Fixture::new();
    let field = f.owner_field();
    let owner = f.new_entry("Owner");
    f.edit(field, owner, None);
    let before = f.read(f.entry.id);
    for value in [None, Some(FieldValue::Text("Wrong storage".into()))] {
        assert!(ProjectService::apply_fields(
            &f.state,
            f.project,
            f.entry.id,
            before.global_revision,
            FieldCommand::SetValues {
                edits: vec![FieldEdit {
                    field_id: field,
                    value
                }]
            }
        )
        .is_err());
        assert_eq!(f.read(f.entry.id), before);
    }
    let backup_root = f.dir.path().join("must-not-be-created");
    assert!(ProjectService::delete_entry_field(
        &f.state,
        f.project,
        f.entry.id,
        field,
        before.global_revision,
        &backup_root
    )
    .is_err());
    assert!(!backup_root.exists());
    let second = f.projection(f.entry.id, f.ownership, Perspective::Target);
    let preview = ProjectService::preview_field_merge(&f.state, f.project, field, second).unwrap();
    assert!(preview
        .blockers
        .iter()
        .any(|s| s.contains("Relationship Fields cannot be merged")));
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM field_value", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        ProjectService::read_relationships(&f.state, f.project, f.entry.id)
            .unwrap()
            .relationships
            .len(),
        1
    );
}

#[test]
fn category_and_parent_type_bindings_make_projections_optional_without_empty_values() {
    let f = Fixture::new();
    let category = ProjectService::create_category(&f.state, f.project, "Objects").unwrap();
    let parent =
        ProjectService::create_type(&f.state, f.project, category.id, None, "Artifact").unwrap();
    let child =
        ProjectService::create_type(&f.state, f.project, category.id, Some(parent.id), "Weapon")
            .unwrap();
    let field = ProjectService::apply_template_fields(
        &f.state,
        f.project,
        f.read(f.entry.id).global_revision,
        FieldCommand::CreateProjection {
            name: "Owner".into(),
            relationship_definition_id: f.ownership,
            perspective: Perspective::Target,
            provider: FieldProvider {
                kind: ProviderKind::Type,
                id: parent.id.to_string(),
            },
        },
    )
    .unwrap()
    .definitions[0]
        .id;
    let object =
        ProjectService::create_entry(&f.state, f.project, Some(category.id), Some(child.id), None)
            .unwrap();
    assert!(f.read(f.entry.id).fields.is_empty());
    let fields = f.read(object.id);
    assert_eq!(fields.fields[0].definition.id, field);
    assert!(fields.fields[0].projected_relationships.is_empty());
    assert_eq!(fields.fields[0].default_sources[0].label, "Artifact");
    f.apply(
        f.entry.id,
        FieldCommand::Bind {
            field_id: field,
            provider: FieldProvider {
                kind: ProviderKind::Category,
                id: f.entry.category_id.to_string(),
            },
        },
    );
    assert_eq!(f.read(f.entry.id).fields.len(), 1);
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM field_value", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn schema_six_upgrade_preserves_populated_children_and_recovers_from_interruption() {
    use worldcrafter_lib::package::{layout, Manifest};
    let dir = tempdir().unwrap();
    let paths = layout::create_skeleton(&dir.path().join("legacy.wcproj")).unwrap();
    let project = ProjectId::new();
    let entry = EntryId::new();
    let category = worldcrafter_lib::domain::CategoryId::new();
    let number = FieldId::new();
    let choice = FieldId::new();
    let option = worldcrafter_lib::domain::structure::ChoiceOptionId::new();
    let conn = Connection::open(paths.db_path()).unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    for sql in [
        include_str!("../src/persistence/migrations/0001_init.sql"),
        include_str!("../src/persistence/migrations/0002_project_structure.sql"),
        include_str!("../src/persistence/migrations/0003_fields.sql"),
        include_str!("../src/persistence/migrations/0004_relationships.sql"),
        include_str!("../src/persistence/migrations/0005_number_units.sql"),
        include_str!("../src/persistence/migrations/0006_entry_field_presentation.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("INSERT INTO project_meta(id,project_id,format_version,schema_version,working_name,created_at,updated_at) VALUES(1,?1,1,6,'Legacy',?2,?2)",params![project.to_string(),now]).unwrap();
    conn.execute(
        "INSERT INTO category VALUES(?1,'Uncategorized',1,?2,?2,1)",
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
    conn.execute("INSERT INTO field_definition VALUES(?1,'Mass','number',NULL,?3,?3,1,'tons'),(?2,'Color','choice',NULL,?3,?3,1,NULL)",params![number.to_string(),choice.to_string(),now]).unwrap();
    for field in [number, choice] {
        conn.execute(
            "INSERT INTO field_availability VALUES(?1,'category',?2)",
            params![field.to_string(), category.to_string()],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO choice_option VALUES(?1,?2,'Blue',NULL,?3,?3,1)",
        params![option.to_string(), choice.to_string(), now],
    )
    .unwrap();
    let number_value = uuid::Uuid::now_v7().to_string();
    let choice_value = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO field_value VALUES(?1,?2,?3,0,'number',NULL,48,NULL,?4,?4,1)",
        params![number_value, entry.to_string(), number.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO field_value VALUES(?1,?2,?3,0,'choice',NULL,NULL,NULL,?4,?4,1)",
        params![choice_value, entry.to_string(), choice.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO field_choice_value VALUES(?1,?2,?3)",
        params![choice_value, choice.to_string(), option.to_string()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO entry_field_presentation VALUES(?1,?2,1,0,?3,?3,1)",
        params![entry.to_string(), number.to_string(), now],
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 6).unwrap();
    Manifest::new(project, 1, 6, "Legacy")
        .write(&paths.manifest_path())
        .unwrap();
    conn.execute_batch("CREATE TRIGGER fail_projection_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected upgrade failure'); END;").unwrap();
    // Exercise the migration connection directly to prove PRAGMAs are restored
    // even after rollback, then exercise the service's validated backup path.
    assert!(worldcrafter_lib::persistence::migrations::migrate(&conn).is_err());
    assert!(conn
        .pragma_query_value(None, "foreign_keys", |r| r.get::<_, bool>(0))
        .unwrap());
    assert!(!conn
        .pragma_query_value(None, "legacy_alter_table", |r| r.get::<_, bool>(0))
        .unwrap());
    drop(conn);
    let state = AppState::default();
    assert!(ProjectService::open_project(&state, &paths.root, false).is_err());
    let conn = Connection::open(paths.db_path()).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        Manifest::read(&paths.manifest_path())
            .unwrap()
            .schema_version,
        6
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='field_projection'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row(
            "SELECT number_value FROM field_value WHERE id=?1",
            [&number_value],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        48.0
    );
    conn.execute_batch("DROP TRIGGER fail_projection_upgrade")
        .unwrap();
    drop(conn);
    ProjectService::open_project(&state, &paths.root, false).unwrap();
    let fields = ProjectService::read_fields(&state, project, entry).unwrap();
    let mass = fields
        .fields
        .iter()
        .find(|f| f.definition.id == number)
        .unwrap();
    assert_eq!(mass.value, Some(FieldValue::Number(48.0)));
    assert_eq!(mass.definition.unit.as_deref(), Some("tons"));
    assert!(mass.hidden);
    assert_eq!(
        fields
            .fields
            .iter()
            .find(|f| f.definition.id == choice)
            .unwrap()
            .value,
        Some(FieldValue::Choices(vec![option]))
    );
    let conn = Connection::open(paths.db_path()).unwrap();
    assert!(!conn
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    assert_eq!(conn.query_row("SELECT count(*) FROM sqlite_master WHERE sql LIKE '%field_definition_before_projections%'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    assert!(conn
        .execute(
            "DELETE FROM field_definition WHERE id=?1",
            [number.to_string()]
        )
        .is_err());
    assert!(conn
        .execute(
            "UPDATE field_definition SET unit='kg' WHERE id=?1",
            [number.to_string()]
        )
        .is_err());
    drop(conn);
    let recovery = Connection::open_with_flags(
        dir.path()
            .join(".worldcrafter-migration-recovery")
            .join(project.to_string())
            .join("schema-v6.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        recovery
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        recovery
            .query_row("SELECT count(*) FROM field_value", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    ProjectService::close_project(&state, project).unwrap();
    assert_eq!(
        Manifest::read(&paths.manifest_path())
            .unwrap()
            .schema_version,
        7
    );
}

#[test]
fn projection_configuration_and_scalar_storage_are_enforced_at_the_boundary() {
    let f = Fixture::new();
    let field = f.owner_field();
    let before = f.read(f.entry.id);
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        FieldCommand::Create {
            name: "Invalid projection".into(),
            field_kind: FieldKind::Relationship,
            unit: None,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: f.entry.id.to_string()
            },
            options: vec![],
            value: None
        }
    )
    .is_err());
    assert!(ProjectService::apply_fields(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        FieldCommand::CreateProjection {
            name: "Wrong Project".into(),
            relationship_definition_id: RelationshipDefinitionId::new(),
            perspective: Perspective::Source,
            provider: FieldProvider {
                kind: ProviderKind::Entry,
                id: f.entry.id.to_string()
            }
        }
    )
    .is_err());
    assert_eq!(f.read(f.entry.id), before);
    let now = chrono::Utc::now().to_rfc3339();
    assert!(f.db().execute("INSERT INTO field_value(id,entry_id,field_id,value_kind,text_value,created_at,updated_at,revision) VALUES(?1,?2,?3,'short_text','Duplicate target',?4,?4,1)",params![uuid::Uuid::now_v7().to_string(),f.entry.id.to_string(),field.to_string(),now]).is_err());
    assert_eq!(f.read(f.entry.id), before);
}

#[test]
fn restored_copy_preserves_projection_ids_and_relationships_but_edits_independently() {
    let f = Fixture::new();
    let field = f.owner_field();
    let owner = f.new_entry("Owner");
    let source = f.edit(field, owner, None);
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let copy = ProjectService::restore_backup_as_copy(
        &f.state,
        &backup,
        f.dir.path(),
        Some("Projection copy"),
    )
    .unwrap();
    assert_ne!(copy.project_id, f.project);
    let restored = ProjectService::read_fields(&f.state, copy.project_id, f.entry.id).unwrap();
    assert_eq!(restored, source);
    let changed = ProjectService::apply_fields(
        &f.state,
        copy.project_id,
        f.entry.id,
        restored.global_revision,
        FieldCommand::EditProjection {
            field_id: field,
            instance_id: None,
            other: OtherEntry::Create {
                name: Some("Copy owner".into()),
                category_id: None,
            },
        },
    )
    .unwrap();
    assert_eq!(
        changed.fields[0].projected_relationships[0].id,
        source.fields[0].projected_relationships[0].id
    );
    assert_eq!(
        changed.fields[0].projected_relationships[0].source.label,
        "Copy owner"
    );
    assert_eq!(f.read(f.entry.id), source);
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}

#[test]
fn concurrent_projection_writes_commit_once_and_never_overwrite_the_first_target() {
    let f = Fixture::new();
    let field = f.owner_field();
    let a = f.new_entry("A");
    let b = f.new_entry("B");
    let expected = f.read(f.entry.id).global_revision;
    let results = std::thread::scope(|scope| {
        let f = &f;
        let handles = [a, b].map(|other| {
            scope.spawn(move || {
                ProjectService::apply_fields(
                    &f.state,
                    f.project,
                    f.entry.id,
                    expected,
                    FieldCommand::EditProjection {
                        field_id: field,
                        other: OtherEntry::Existing { id: other },
                        instance_id: None,
                    },
                )
            })
        });
        handles.map(|handle| handle.join().unwrap())
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .find_map(|r| r.as_ref().err())
        .unwrap()
        .to_string()
        .to_lowercase()
        .contains("revision"));
    let saved = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(f.read(f.entry.id), saved);
    assert_eq!(saved.fields[0].projected_relationships.len(), 1);
}

#[test]
fn unresolved_participant_remains_visible_and_retarget_repairs_only_selected_connection() {
    let f = Fixture::new();
    let field = f.owner_field();
    let owner = f.new_entry("Former owner");
    let id = f.connect(owner, vec![]).relationships[0].id;
    f.db().execute("UPDATE relationship_participant SET record_id=NULL,unresolved_snapshot=?1 WHERE instance_id=?2 AND slot='source'",params![serde_json::json!({"former_id":owner.to_string(),"kind":"entry","label":"Former owner"}).to_string(),id.to_string()]).unwrap();
    let before = f.read(f.entry.id);
    let missing = &before.fields[0].projected_relationships[0].source;
    assert_eq!(missing.id, None);
    assert_eq!(missing.workspace_state, "missing");
    assert_eq!(missing.label, "Former owner");
    let other = f.new_entry("New owner");
    let after = f.edit(field, other, Some(id));
    assert_eq!(after.fields[0].projected_relationships[0].id, id);
    assert_eq!(
        after.fields[0].projected_relationships[0].note,
        "Preserved authored note"
    );
    assert_eq!(
        after.fields[0].projected_relationships[0].source.id,
        Some(other)
    );
}
