use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        relationships::*,
        structure::{RelationshipDefinitionId, RelationshipId},
        Entry, EntryId, ProjectId,
    },
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
        let project =
            ProjectService::create_project(&state, dir.path(), "Relationships Test").unwrap();
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
    fn read(&self, entry: EntryId) -> EntryRelationships {
        ProjectService::read_relationships(&self.state, self.project, entry).unwrap()
    }
    fn apply(&self, entry: EntryId, command: RelationshipCommand) -> EntryRelationships {
        ProjectService::apply_relationships(
            &self.state,
            self.project,
            entry,
            self.read(entry).global_revision,
            command,
        )
        .unwrap()
    }
    fn definition(&self, draft: DefinitionDraft) -> RelationshipDefinitionId {
        self.apply(
            self.entry.id,
            RelationshipCommand::CreateDefinition { draft },
        )
        .definitions
        .iter()
        .find(|d| d.draft.name == "Ownership")
        .unwrap()
        .id
    }
    fn connect(
        &self,
        entry: EntryId,
        definition_id: RelationshipDefinitionId,
        other: OtherEntry,
        replace: Vec<RelationshipId>,
    ) -> EntryRelationships {
        self.apply(
            entry,
            RelationshipCommand::Connect {
                definition_id,
                perspective: Perspective::Source,
                other,
                note: "Authored note".into(),
                replace,
            },
        )
    }
    fn new_entry(&self, name: &str) -> Entry {
        ProjectService::create_entry(&self.state, self.project, None, None, Some(name.into()))
            .unwrap()
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
fn ownership() -> DefinitionDraft {
    DefinitionDraft {
        name: "Ownership".into(),
        forward_label: "owns".into(),
        inverse_label: "is owned by".into(),
        directed: true,
        expected_targets_per_source: None,
        expected_sources_per_target: Some(1),
    }
}
fn stub() -> OtherEntry {
    OtherEntry::Create {
        name: Some("Singularity Blade".into()),
        category_id: None,
    }
}

#[test]
fn one_record_has_two_views_stable_identity_notes_and_reversible_history() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let source = f.connect(f.entry.id, d, stub(), vec![]);
    let r = &source.relationships[0];
    let blade = r.target.id.unwrap();
    assert_eq!(f.read(blade).relationships[0], *r);
    let conn = f.db();
    let count: i64 = conn
        .query_row("SELECT count(*) FROM relationship_instance", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
    let participants: i64 = conn
        .query_row("SELECT count(*) FROM relationship_participant", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(participants, 2);
    let renamed = ProjectService::update_entry_name(
        &f.state,
        f.project,
        f.entry.id,
        1,
        Some("Thron renamed".into()),
    )
    .unwrap();
    let category = ProjectService::create_category(&f.state, f.project, "People").unwrap();
    ProjectService::change_entry_structure(
        &f.state,
        f.project,
        renamed.id,
        renamed.revision,
        category.id,
        None,
    )
    .unwrap();
    let inverse = f.read(blade);
    assert_eq!(inverse.relationships[0].source.label, "Thron renamed");
    assert_eq!(inverse.relationships[0].id, r.id);
    f.apply(
        blade,
        RelationshipCommand::SetNote {
            id: r.id,
            note: "New note from the inverse side".into(),
        },
    );
    let ended = f.apply(
        blade,
        RelationshipCommand::SetEnded {
            id: r.id,
            ended: true,
        },
    );
    assert!(ended.relationships[0].ended);
    let restored = f.apply(
        f.entry.id,
        RelationshipCommand::SetEnded {
            id: r.id,
            ended: false,
        },
    );
    assert_eq!(restored.relationships[0].id, r.id);
    assert_eq!(
        restored.relationships[0].note,
        "New note from the inverse side"
    );
    assert!(!restored.relationships[0].ended);
}

#[test]
fn conflicts_remain_visible_replacement_is_explicit_and_transactional() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let first = f.connect(f.entry.id, d, stub(), vec![]);
    let original = &first.relationships[0];
    let blade = original.target.id.unwrap();
    let second = f.new_entry("Another owner");
    f.connect(second.id, d, OtherEntry::Existing { id: blade }, vec![]);
    let conflict = f.read(blade);
    assert_eq!(conflict.relationships.len(), 2);
    assert!(conflict
        .relationships
        .iter()
        .all(|r| !r.warnings.is_empty() && !r.ended));
    let ids: Vec<_> = conflict.relationships.iter().map(|r| r.id).collect();
    let result = f.apply(
        blade,
        RelationshipCommand::Connect {
            definition_id: d,
            perspective: Perspective::Target,
            other: OtherEntry::Create {
                name: None,
                category_id: None,
            },
            note: "Resolved explicitly".into(),
            replace: ids.clone(),
        },
    );
    assert_eq!(result.relationships.len(), 3);
    assert_eq!(result.relationships.iter().filter(|r| r.ended).count(), 2);
    assert!(result.relationships.iter().all(|r| r.warnings.is_empty()));
    assert!(result
        .relationships
        .iter()
        .filter(|r| ids.contains(&r.id))
        .all(|r| r.note == "Authored note"));
    let conn = f.db();
    conn.execute_batch("CREATE TRIGGER fail_link BEFORE INSERT ON relationship_participant WHEN NEW.slot='target' BEGIN SELECT RAISE(ABORT,'simulated disk failure'); END;").unwrap();
    let before = f.read(blade);
    let current = before.relationships.iter().find(|r| !r.ended).unwrap().id;
    assert!(ProjectService::apply_relationships(
        &f.state,
        f.project,
        blade,
        before.global_revision,
        RelationshipCommand::Connect {
            definition_id: d,
            perspective: Perspective::Target,
            other: stub(),
            note: "must not commit".into(),
            replace: vec![current]
        }
    )
    .is_err());
    assert_eq!(f.read(blade), before); // Includes entry list: no orphan quick-created Entry.
}

#[test]
fn duplicate_and_symmetric_connections_are_not_duplicated_but_self_connections_are_valid() {
    let f = Fixture::new();
    let mut draft = ownership();
    draft.directed = false;
    draft.forward_label = "allied with".into();
    draft.inverse_label = draft.forward_label.clone();
    draft.expected_sources_per_target = None;
    let d = f.definition(draft);
    let r = f
        .connect(f.entry.id, d, stub(), vec![])
        .relationships
        .remove(0);
    let other = r
        .target
        .id
        .filter(|id| *id != f.entry.id)
        .unwrap_or(r.source.id.unwrap());
    let before = f.read(other);
    assert!(ProjectService::apply_relationships(
        &f.state,
        f.project,
        other,
        before.global_revision,
        RelationshipCommand::Connect {
            definition_id: d,
            perspective: Perspective::Source,
            other: OtherEntry::Existing { id: f.entry.id },
            note: String::new(),
            replace: vec![]
        }
    )
    .is_err());
    assert_eq!(f.read(other), before);
    let self_connection = f.connect(
        f.entry.id,
        d,
        OtherEntry::Existing { id: f.entry.id },
        vec![],
    );
    assert_eq!(self_connection.relationships.len(), 2);
    f.apply(
        f.entry.id,
        RelationshipCommand::SetEnded {
            id: r.id,
            ended: true,
        },
    );
    f.connect(f.entry.id, d, OtherEntry::Existing { id: other }, vec![]);
    assert!(ProjectService::apply_relationships(
        &f.state,
        f.project,
        other,
        f.read(other).global_revision,
        RelationshipCommand::SetEnded {
            id: r.id,
            ended: false
        }
    )
    .is_err());
}

#[test]
fn definition_changes_preserve_instances_and_retirement_only_stops_new_use() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let r = f
        .connect(f.entry.id, d, stub(), vec![])
        .relationships
        .remove(0);
    let mut renamed = ownership();
    renamed.name = "Possession".into();
    renamed.inverse_label = "belongs to".into();
    let result = f.apply(
        f.entry.id,
        RelationshipCommand::UpdateDefinition {
            definition_id: d,
            draft: renamed,
        },
    );
    assert_eq!(result.definitions[0].id, d);
    assert_eq!(result.relationships[0], r);
    f.apply(
        f.entry.id,
        RelationshipCommand::RetireDefinition {
            definition_id: d,
            retired: true,
        },
    );
    assert_eq!(f.read(f.entry.id).relationships[0], r);
    let before = f.read(f.entry.id);
    assert!(ProjectService::apply_relationships(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        RelationshipCommand::Connect {
            definition_id: d,
            perspective: Perspective::Source,
            other: stub(),
            note: String::new(),
            replace: vec![]
        }
    )
    .is_err());
    assert_eq!(f.read(f.entry.id), before);
    f.apply(
        f.entry.id,
        RelationshipCommand::SetNote {
            id: r.id,
            note: "still editable".into(),
        },
    );
    f.apply(
        f.entry.id,
        RelationshipCommand::RetireDefinition {
            definition_id: d,
            retired: false,
        },
    );
    f.connect(f.entry.id, d, stub(), vec![]);
}

#[test]
fn stale_invalid_and_foreign_operations_leave_every_record_unchanged() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let foreign = Fixture::new();
    let before = f.read(f.entry.id);
    for (entry, revision, command) in [
        (
            f.entry.id,
            before.global_revision - 1,
            RelationshipCommand::Connect {
                definition_id: d,
                perspective: Perspective::Source,
                other: stub(),
                note: String::new(),
                replace: vec![],
            },
        ),
        (
            f.entry.id,
            before.global_revision,
            RelationshipCommand::Connect {
                definition_id: d,
                perspective: Perspective::Source,
                other: OtherEntry::Existing {
                    id: foreign.entry.id,
                },
                note: String::new(),
                replace: vec![],
            },
        ),
        (
            foreign.entry.id,
            before.global_revision,
            RelationshipCommand::CreateDefinition { draft: ownership() },
        ),
        (
            f.entry.id,
            before.global_revision,
            RelationshipCommand::SetNote {
                id: RelationshipId::new(),
                note: "foreign".into(),
            },
        ),
    ] {
        assert!(
            ProjectService::apply_relationships(&f.state, f.project, entry, revision, command)
                .is_err()
        );
        assert_eq!(f.read(f.entry.id), before);
    }
    let mut zero = ownership();
    zero.expected_sources_per_target = Some(0);
    let mut asymmetric = ownership();
    asymmetric.directed = false;
    for draft in [zero, asymmetric] {
        assert!(ProjectService::apply_relationships(
            &f.state,
            f.project,
            f.entry.id,
            before.global_revision,
            RelationshipCommand::CreateDefinition { draft }
        )
        .is_err());
    }
    let conn = f.db();
    assert!(conn.execute("INSERT INTO relationship_instance(id,definition_id,semantic_state,note,created_at,updated_at,revision) VALUES(?1,?2,'active','','now','now',1)",params![f.entry.id.to_string(),d.to_string()]).is_err());
    assert_eq!(f.read(f.entry.id), before);
}

#[test]
fn competing_commands_have_one_commit_and_one_stale_failure() {
    let f = Fixture::new();
    let revision = f.read(f.entry.id).global_revision;
    std::thread::scope(|scope| {
        let run = || {
            ProjectService::apply_relationships(
                &f.state,
                f.project,
                f.entry.id,
                revision,
                RelationshipCommand::CreateDefinition { draft: ownership() },
            )
        };
        let one = scope.spawn(run);
        let two = scope.spawn(run);
        let outcomes = [one.join().unwrap(), two.join().unwrap()];
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(outcomes.iter().filter(|r| r.is_err()).count(), 1);
    });
    let snapshot = f.read(f.entry.id);
    assert_eq!(snapshot.global_revision, revision + 1);
    assert_eq!(snapshot.definitions.len(), 1);
}

#[test]
fn missing_participant_stays_visible_and_cannot_be_silently_restored() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let r = f
        .connect(f.entry.id, d, stub(), vec![])
        .relationships
        .remove(0);
    let conn = f.db();
    // Synthetic future-deletion fixture: no supplied Projects are opened here.
    conn.execute("UPDATE relationship_participant SET record_id=NULL,unresolved_snapshot=?1 WHERE instance_id=?2 AND slot='target'",
        params![r#"{"label":"Former Blade"}"#,r.id.to_string()]).unwrap();
    let missing = f.read(f.entry.id);
    assert_eq!(missing.relationships[0].target.id, None);
    assert_eq!(missing.relationships[0].target.label, "Former Blade");
    assert_eq!(missing.relationships[0].target.workspace_state, "missing");
    f.apply(
        f.entry.id,
        RelationshipCommand::SetEnded {
            id: r.id,
            ended: true,
        },
    );
    let before = f.read(f.entry.id);
    assert!(ProjectService::apply_relationships(
        &f.state,
        f.project,
        f.entry.id,
        before.global_revision,
        RelationshipCommand::SetEnded {
            id: r.id,
            ended: false
        }
    )
    .is_err());
    assert_eq!(f.read(f.entry.id), before);
}

#[test]
fn backup_restore_and_reopen_preserve_both_participants_and_internal_ids() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let expected = f.connect(f.entry.id, d, stub(), vec![]);
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(f.read(f.entry.id), expected);
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
    let restored =
        ProjectService::read_relationships(&f.state, copy.project_id, f.entry.id).unwrap();
    assert_eq!(restored, expected);
    ProjectService::close_project(&f.state, copy.project_id).unwrap();
}

#[test]
fn schema_three_upgrade_is_backed_up_and_failed_upgrade_can_be_retried() {
    let f = Fixture::new();
    ProjectService::close_project(&f.state, f.project).unwrap();
    let conn = f.db();
    conn.execute_batch("DROP TABLE entry_field_presentation; DROP TRIGGER field_unit_preserve_values; ALTER TABLE field_definition DROP COLUMN unit; DROP TABLE relationship_participant; DROP TABLE relationship_instance; DROP TABLE relationship_definition; PRAGMA user_version=3; UPDATE project_meta SET schema_version=3;").unwrap();
    let manifest_path = std::path::Path::new(&f.path).join("manifest.json");
    let mut manifest = Manifest::read(&manifest_path).unwrap();
    manifest.schema_version = 3;
    manifest.write(&manifest_path).unwrap();
    conn.execute_batch("CREATE TABLE relationship_definition (unexpected TEXT);")
        .unwrap();
    drop(conn);
    assert!(ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).is_err());
    let conn = f.db();
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 3);
    assert_eq!(Manifest::read(&manifest_path).unwrap().schema_version, 3);
    conn.execute_batch("DROP TABLE relationship_definition;")
        .unwrap();
    drop(conn);
    let opened =
        ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(
        opened.schema_version,
        worldcrafter_lib::persistence::migrations::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(f.read(f.entry.id).entries[0].id, f.entry.id);
    let recovery = f
        .dir
        .path()
        .join(".worldcrafter-migration-recovery")
        .join(f.project.to_string())
        .join("schema-v3.sqlite");
    let conn =
        Connection::open_with_flags(recovery, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row(
            "SELECT authored_name FROM entry WHERE id=?1",
            [f.entry.id.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "Thron"
    );
}

#[test]
fn project_browser_reads_each_canonical_connection_once_including_past_and_conflicts() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let owner2 = f.new_entry("Another owner");
    let target = f.new_entry("Object");
    f.connect(
        f.entry.id,
        d,
        OtherEntry::Existing { id: target.id },
        vec![],
    );
    f.connect(owner2.id, d, OtherEntry::Existing { id: target.id }, vec![]);
    let ended = f
        .connect(
            owner2.id,
            d,
            OtherEntry::Existing { id: f.entry.id },
            vec![],
        )
        .relationships
        .into_iter()
        .find(|r| r.target.id == Some(f.entry.id))
        .unwrap()
        .id;
    f.apply(
        owner2.id,
        RelationshipCommand::SetEnded {
            id: ended,
            ended: true,
        },
    );
    let symmetric = f
        .apply(
            f.entry.id,
            RelationshipCommand::CreateDefinition {
                draft: DefinitionDraft {
                    name: "Alliance".into(),
                    forward_label: "allied with".into(),
                    inverse_label: "allied with".into(),
                    directed: false,
                    expected_sources_per_target: None,
                    expected_targets_per_source: None,
                },
            },
        )
        .definitions
        .into_iter()
        .find(|d| d.draft.name == "Alliance")
        .unwrap()
        .id;
    f.connect(
        f.entry.id,
        symmetric,
        OtherEntry::Existing { id: owner2.id },
        vec![],
    );
    f.connect(
        f.entry.id,
        symmetric,
        OtherEntry::Existing { id: f.entry.id },
        vec![],
    );
    let before = f.read(f.entry.id);
    let snapshot = ProjectService::read_project_relationships(&f.state, f.project).unwrap();
    assert_eq!(snapshot.global_revision, before.global_revision);
    assert_eq!(snapshot.relationships.len(), 5);
    assert_eq!(
        snapshot
            .relationships
            .iter()
            .map(|r| r.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        5
    );
    assert_eq!(
        snapshot
            .relationships
            .iter()
            .filter(|r| !r.warnings.is_empty())
            .count(),
        2
    );
    assert_eq!(snapshot.relationships.iter().filter(|r| r.ended).count(), 1);
    for entry in [f.entry.id, owner2.id, target.id] {
        let projected: Vec<_> = snapshot
            .relationships
            .iter()
            .filter(|r| r.involves(entry))
            .cloned()
            .collect();
        assert_eq!(projected, f.read(entry).relationships);
    }
    assert_eq!(
        f.read(f.entry.id),
        before,
        "Browsing must not write or advance revisions"
    );
}

#[test]
fn project_browser_preserves_unavailable_participants_and_is_project_scoped() {
    let f = Fixture::new();
    let d = f.definition(ownership());
    let r = f
        .connect(f.entry.id, d, stub(), vec![])
        .relationships
        .remove(0);
    // Disposable future-deletion fixture; keep missing participants visible.
    f.db().execute("UPDATE relationship_participant SET record_id=NULL,unresolved_snapshot=?1 WHERE instance_id=?2 AND slot='target'",
        params![r#"{"label":"Former object"}"#, r.id.to_string()]).unwrap();
    f.apply(
        f.entry.id,
        RelationshipCommand::RetireDefinition {
            definition_id: d,
            retired: true,
        },
    );
    let snapshot = ProjectService::read_project_relationships(&f.state, f.project).unwrap();
    assert_eq!(snapshot.relationships[0].target.label, "Former object");
    assert_eq!(snapshot.relationships[0].target.id, None);
    assert_eq!(snapshot.relationships[0].target.workspace_state, "missing");
    assert!(snapshot.definitions[0].retired);
    let other = ProjectService::create_project(&f.state, f.dir.path(), "Empty Project").unwrap();
    let empty = ProjectService::read_project_relationships(&f.state, other.project_id).unwrap();
    assert!(empty.relationships.is_empty());
    assert!(empty.entries.is_empty());
    assert!(empty.definitions.is_empty());
    ProjectService::close_project(&f.state, other.project_id).unwrap();
    assert!(ProjectService::read_project_relationships(&f.state, other.project_id).is_err());
    assert!(ProjectService::read_project_relationships(&f.state, ProjectId::new()).is_err());
    assert_eq!(
        ProjectService::read_project_relationships(&f.state, f.project).unwrap(),
        snapshot
    );
}
