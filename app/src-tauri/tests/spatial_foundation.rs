use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    domain::{relationships::*, spatial::*, CategoryId, EntryId, ProjectId, TypeId},
    persistence::{worker::InitialProjectMeta, PersistenceError, ProjectDbWorker},
};

struct Fixture {
    worker: ProjectDbWorker,
    dir: TempDir,
    project: ProjectId,
}

#[test]
fn abrupt_process_exit_keeps_committed_moves_and_rolls_back_uncommitted_moves() {
    let f = Fixture::new();
    let a = f.entry("A", true);
    let b = f.entry("B", true);
    let child = f.entry("Child", true);
    f.parent(child, Some(a));
    let path = f.dir.path().join("project.sqlite");
    f.worker.shutdown().unwrap();
    for (mode, parent) in [("committed", b), ("uncommitted", a)] {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "spatial_process_exit_helper", "--ignored"])
            .env("WORLDCRAFTER_SPATIAL_TEST_DB", &path)
            .env("WORLDCRAFTER_SPATIAL_TEST_PROJECT", f.project.to_string())
            .env("WORLDCRAFTER_SPATIAL_TEST_CHILD", child.to_string())
            .env("WORLDCRAFTER_SPATIAL_TEST_PARENT", parent.to_string())
            .env("WORLDCRAFTER_SPATIAL_TEST_MODE", mode)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        let worker = ProjectDbWorker::spawn(path.clone(), f.project, None).unwrap();
        assert_eq!(
            worker
                .read_spatial()
                .unwrap()
                .entries
                .iter()
                .find(|e| e.id == child)
                .unwrap()
                .parent_id,
            Some(b)
        );
        worker.shutdown().unwrap();
    }
}

#[test]
#[ignore = "subprocess helper invoked by the abrupt-exit integration test"]
fn spatial_process_exit_helper() {
    let path = std::path::PathBuf::from(std::env::var("WORLDCRAFTER_SPATIAL_TEST_DB").unwrap());
    let child = EntryId::parse(&std::env::var("WORLDCRAFTER_SPATIAL_TEST_CHILD").unwrap()).unwrap();
    let parent =
        EntryId::parse(&std::env::var("WORLDCRAFTER_SPATIAL_TEST_PARENT").unwrap()).unwrap();
    if std::env::var("WORLDCRAFTER_SPATIAL_TEST_MODE").unwrap() == "committed" {
        let project =
            ProjectId::parse(&std::env::var("WORLDCRAFTER_SPATIAL_TEST_PROJECT").unwrap()).unwrap();
        let worker = ProjectDbWorker::spawn(path, project, None).unwrap();
        worker
            .apply_spatial(
                worker.read_spatial().unwrap().global_revision,
                SpatialCommand::Reparent {
                    entry_id: child,
                    parent_id: Some(parent),
                },
            )
            .unwrap();
        // Exit without destructors, worker shutdown, or a WAL checkpoint.
        std::process::exit(0);
    }
    let conn = Connection::open(path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE")
        .unwrap();
    conn.execute(
        "UPDATE spatial_node SET primary_parent_id=?1 WHERE entry_id=?2",
        params![parent.to_string(), child.to_string()],
    )
    .unwrap();
    std::process::exit(0);
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let project = ProjectId::new();
        let worker = ProjectDbWorker::spawn(
            dir.path().join("project.sqlite"),
            project,
            Some(InitialProjectMeta {
                working_name: "Spatial test".into(),
                format_version: 1,
            }),
        )
        .unwrap();
        Self {
            worker,
            dir,
            project,
        }
    }
    fn db(&self) -> Connection {
        let c = Connection::open(self.dir.path().join("project.sqlite")).unwrap();
        c.pragma_update(None, "foreign_keys", true).unwrap();
        c
    }
    fn read(&self) -> SpatialSnapshot {
        self.worker.read_spatial().unwrap()
    }
    fn apply(&self, command: SpatialCommand) -> SpatialSnapshot {
        self.worker
            .apply_spatial(self.read().global_revision, command)
            .unwrap()
    }
    fn entry(&self, name: &str, spatial: bool) -> EntryId {
        let id = EntryId::new();
        self.worker
            .create_entry(id, None, None, Some(name.into()))
            .unwrap();
        if spatial {
            self.apply(SpatialCommand::SetEnabled {
                entry_id: id,
                enabled: true,
            });
        }
        id
    }
    fn parent(&self, child: EntryId, parent: Option<EntryId>) {
        self.apply(SpatialCommand::Reparent {
            entry_id: child,
            parent_id: parent,
        });
    }
    fn ancestry(&self, id: EntryId) -> Vec<String> {
        let db = self.db();
        let mut stmt=db.prepare("WITH RECURSIVE ancestors(id,depth) AS (SELECT ?1,0 UNION ALL SELECT s.primary_parent_id,a.depth+1 FROM spatial_node s JOIN ancestors a ON s.entry_id=a.id WHERE s.primary_parent_id IS NOT NULL) SELECT e.authored_name FROM ancestors a JOIN entry e ON e.id=a.id ORDER BY depth DESC").unwrap();
        stmt.query_map([id.to_string()], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }
}

#[test]
fn tortuga_reparent_changes_only_araks_parent_not_temple_or_throns_direct_location() {
    let f = Fixture::new();
    let tortuga = f.entry("Tortuga", true);
    let shell = f.entry("Northern Shell", true);
    let arak = f.entry("Arak", true);
    let temple = f.entry("Temple", true);
    let thron = f.entry("Thron", false);
    let continent = f.entry("Floating Continent", true);
    f.parent(shell, Some(tortuga));
    f.parent(arak, Some(shell));
    f.parent(temple, Some(arak));
    let def = f
        .worker
        .apply_relationships(
            thron,
            f.read().global_revision,
            RelationshipCommand::CreateDefinition {
                draft: DefinitionDraft {
                    name: "Current Location".into(),
                    forward_label: "is at".into(),
                    inverse_label: "hosts".into(),
                    directed: true,
                    expected_targets_per_source: Some(1),
                    expected_sources_per_target: None,
                },
            },
        )
        .unwrap()
        .definitions[0]
        .id;
    let direct = f
        .worker
        .apply_relationships(
            thron,
            f.read().global_revision,
            RelationshipCommand::Connect {
                definition_id: def,
                perspective: Perspective::Source,
                other: OtherEntry::Existing { id: temple },
                note: "Inside the sanctuary".into(),
                replace: vec![],
            },
        )
        .unwrap()
        .relationships;
    let temple_before: (Option<String>, String, i64) = f
        .db()
        .query_row(
            "SELECT primary_parent_id,updated_at,revision FROM spatial_node WHERE entry_id=?1",
            [temple.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    let thron_before = f.worker.get_entry(thron).unwrap();
    assert_eq!(
        f.ancestry(temple),
        ["Tortuga", "Northern Shell", "Arak", "Temple"]
    );
    f.parent(arak, Some(continent));
    assert_eq!(f.ancestry(temple), ["Floating Continent", "Arak", "Temple"]);
    assert_eq!(
        f.worker.read_relationships(thron).unwrap().relationships,
        direct
    );
    let thron_after = f.worker.get_entry(thron).unwrap();
    assert_eq!(thron_before.revision, thron_after.revision);
    assert_eq!(thron_before.id, thron_after.id);
    assert_eq!(
        temple_before,
        f.db()
            .query_row(
                "SELECT primary_parent_id,updated_at,revision FROM spatial_node WHERE entry_id=?1",
                [temple.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            )
            .unwrap()
    );
    f.parent(arak, Some(shell));
    assert_eq!(
        f.ancestry(temple),
        ["Tortuga", "Northern Shell", "Arak", "Temple"]
    );
    let before = serde_json::to_value(f.read()).unwrap();
    for parent in [temple, tortuga] {
        assert!(f
            .worker
            .apply_spatial(
                f.read().global_revision,
                SpatialCommand::Reparent {
                    entry_id: tortuga,
                    parent_id: Some(parent)
                }
            )
            .is_err());
        assert_eq!(serde_json::to_value(f.read()).unwrap(), before);
    }
    f.worker
        .backup_to(f.dir.path().join("snapshot.sqlite"))
        .unwrap();
    let backup = Connection::open(f.dir.path().join("snapshot.sqlite")).unwrap();
    assert_eq!(
        backup
            .query_row("SELECT count(*) FROM spatial_node", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        5
    );
    let path = f.dir.path().join("project.sqlite");
    f.worker.shutdown().unwrap();
    let reopened = ProjectDbWorker::spawn(path, f.project, None).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.read_spatial().unwrap()).unwrap(),
        before
    );
    assert_eq!(
        reopened.read_relationships(thron).unwrap().relationships,
        direct
    );
}

#[test]
fn defaults_union_ancestor_types_at_creation_and_never_reclassify_or_change_existing_features() {
    let f = Fixture::new();
    let category = CategoryId::new();
    let root = TypeId::new();
    let leaf = TypeId::new();
    f.worker
        .create_category(category, "Creatures".into())
        .unwrap();
    f.worker
        .create_type(root, category, None, "World Animal".into())
        .unwrap();
    f.worker
        .create_type(leaf, category, Some(root), "World Turtle".into())
        .unwrap();
    let old = EntryId::new();
    f.worker
        .create_entry(old, Some(category), Some(leaf), None)
        .unwrap();
    let provider = CapabilityProvider::Type { id: root };
    f.apply(SpatialCommand::SetDefault {
        provider: provider.clone(),
        enabled: true,
    });
    let tortuga = EntryId::new();
    f.worker
        .create_entry(tortuga, Some(category), Some(leaf), Some("Tortuga".into()))
        .unwrap();
    f.apply(SpatialCommand::SetDefault {
        provider: CapabilityProvider::Category { id: category },
        enabled: true,
    });
    let union = EntryId::new();
    f.worker
        .create_entry(union, Some(category), Some(leaf), None)
        .unwrap();
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM entry_capability WHERE entry_id=?1",
                [union.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
    f.apply(SpatialCommand::SetDefault {
        provider,
        enabled: false,
    });
    f.apply(SpatialCommand::SetDefault {
        provider: CapabilityProvider::Category { id: category },
        enabled: false,
    });
    assert!(
        !f.read()
            .entries
            .iter()
            .find(|e| e.id == old)
            .unwrap()
            .spatial
    );
    assert!(
        f.read()
            .entries
            .iter()
            .find(|e| e.id == tortuga)
            .unwrap()
            .spatial
    );
    let entry = f.worker.get_entry(tortuga).unwrap();
    assert_eq!(entry.category_id, category);
    assert_eq!(entry.type_id, Some(leaf));
    let uncategorized = f
        .worker
        .list_categories()
        .unwrap()
        .into_iter()
        .find(|c| c.is_uncategorized)
        .unwrap()
        .id;
    f.worker
        .change_entry_structure(tortuga, entry.revision, uncategorized, None)
        .unwrap();
    assert!(
        f.read()
            .entries
            .iter()
            .find(|e| e.id == tortuga)
            .unwrap()
            .spatial
    );
    let future = EntryId::new();
    f.worker
        .create_entry(future, Some(category), Some(leaf), None)
        .unwrap();
    assert!(
        !f.read()
            .entries
            .iter()
            .find(|e| e.id == future)
            .unwrap()
            .spatial
    );
}

#[test]
fn relationship_quick_creation_materializes_category_features_too() {
    let f = Fixture::new();
    let source = f.entry("Visitor", false);
    let category = CategoryId::new();
    f.worker.create_category(category, "Places".into()).unwrap();
    f.apply(SpatialCommand::SetDefault {
        provider: CapabilityProvider::Category { id: category },
        enabled: true,
    });
    let def = f
        .worker
        .apply_relationships(
            source,
            f.read().global_revision,
            RelationshipCommand::CreateDefinition {
                draft: DefinitionDraft {
                    name: "Visit".into(),
                    forward_label: "visits".into(),
                    inverse_label: "visited by".into(),
                    directed: true,
                    expected_targets_per_source: None,
                    expected_sources_per_target: None,
                },
            },
        )
        .unwrap()
        .definitions[0]
        .id;
    f.worker
        .apply_relationships(
            source,
            f.read().global_revision,
            RelationshipCommand::Connect {
                definition_id: def,
                perspective: Perspective::Source,
                other: OtherEntry::Create {
                    name: Some("Harbor".into()),
                    category_id: Some(category),
                },
                note: String::new(),
                replace: vec![],
            },
        )
        .unwrap();
    assert!(
        f.read()
            .entries
            .iter()
            .find(|e| e.label == "Harbor")
            .unwrap()
            .spatial
    );
}

#[test]
fn removal_requires_explicit_repair_and_base_cannot_be_removed() {
    let f = Fixture::new();
    let a = f.entry("Parent", true);
    let b = f.entry("Child", true);
    f.parent(b, Some(a));
    for entry_id in [a, b] {
        assert!(f
            .worker
            .apply_spatial(
                f.read().global_revision,
                SpatialCommand::SetEnabled {
                    entry_id,
                    enabled: false
                }
            )
            .is_err());
    }
    assert!(f
        .db()
        .execute(
            "DELETE FROM entry_capability WHERE entry_id=?1 AND capability_id='base'",
            [a.to_string()]
        )
        .is_err());
    assert!(f
        .db()
        .execute(
            "DELETE FROM entry_capability WHERE entry_id=?1 AND capability_id='spatial'",
            [a.to_string()]
        )
        .is_err());
    assert!(f
        .db()
        .execute(
            "DELETE FROM spatial_node WHERE entry_id=?1",
            [a.to_string()]
        )
        .is_err());
    f.parent(b, None);
    f.apply(SpatialCommand::SetEnabled {
        entry_id: a,
        enabled: false,
    });
    assert_eq!(f.worker.get_entry(a).unwrap().id, a);
    assert!(!f.read().entries.iter().find(|e| e.id == a).unwrap().spatial);
}

#[test]
fn foreign_non_spatial_inactive_and_stale_moves_are_rejected_without_writes() {
    let f = Fixture::new();
    let a = f.entry("A", true);
    let non = f.entry("No Spatial", false);
    let inactive = f.entry("Archived", true);
    f.db()
        .execute(
            "UPDATE record_identity SET workspace_state='archived' WHERE record_id=?1",
            [inactive.to_string()],
        )
        .unwrap();
    let before = serde_json::to_value(f.read()).unwrap();
    for parent_id in [non, inactive, EntryId::new()] {
        assert!(f
            .worker
            .apply_spatial(
                f.read().global_revision,
                SpatialCommand::Reparent {
                    entry_id: a,
                    parent_id: Some(parent_id)
                }
            )
            .is_err());
    }
    assert!(matches!(
        f.worker.apply_spatial(
            0,
            SpatialCommand::Reparent {
                entry_id: a,
                parent_id: None
            }
        ),
        Err(PersistenceError::StaleRevision { .. })
    ));
    assert_eq!(serde_json::to_value(f.read()).unwrap(), before);
}

#[test]
fn failures_roll_back_the_whole_move_or_child_creation_and_can_retry() {
    let f = Fixture::new();
    let a = f.entry("A", true);
    let b = f.entry("B", true);
    let before = serde_json::to_value(f.read()).unwrap();
    f.db().execute_batch("CREATE TRIGGER injected_failure BEFORE UPDATE OF last_committed_revision ON project_meta BEGIN SELECT RAISE(ABORT,'injected publication failure'); END;").unwrap();
    let command = SpatialCommand::CreateChild {
        parent_id: a,
        name: Some("New child".into()),
        category_id: None,
        type_id: None,
    };
    assert!(f
        .worker
        .apply_spatial(f.read().global_revision, command.clone())
        .is_err());
    assert!(f
        .worker
        .apply_spatial(
            f.read().global_revision,
            SpatialCommand::Reparent {
                entry_id: b,
                parent_id: Some(a)
            }
        )
        .is_err());
    assert_eq!(serde_json::to_value(f.read()).unwrap(), before);
    f.db()
        .execute_batch("DROP TRIGGER injected_failure")
        .unwrap();
    f.apply(command);
    assert_eq!(f.read().entries.len(), 3);
    let child = f
        .read()
        .entries
        .into_iter()
        .find(|e| e.label == "New child")
        .unwrap();
    assert_eq!(child.parent_id, Some(a));
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM record_identity WHERE kind='entry'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        3
    );
    assert!(!f
        .db()
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
}

#[test]
fn two_concurrent_moves_cannot_both_commit_against_the_same_revision() {
    let f = Fixture::new();
    let a = f.entry("A", true);
    let b = f.entry("B", true);
    let expected = f.read().global_revision;
    std::thread::scope(|scope| {
        let worker = &f.worker;
        let one = scope.spawn(move || {
            worker.apply_spatial(
                expected,
                SpatialCommand::Reparent {
                    entry_id: a,
                    parent_id: Some(b),
                },
            )
        });
        let two = scope.spawn(move || {
            worker.apply_spatial(
                expected,
                SpatialCommand::Reparent {
                    entry_id: b,
                    parent_id: Some(a),
                },
            )
        });
        let results = [one.join().unwrap(), two.join().unwrap()];
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| matches!(r, Err(PersistenceError::StaleRevision { .. })))
                .count(),
            1
        );
    });
}

#[test]
fn schema_seven_migration_is_atomic_backed_up_and_preserves_existing_entries() {
    use worldcrafter_lib::{
        application::{AppState, ProjectService},
        package::manifest::Manifest,
    };
    let dir = tempdir().unwrap();
    let state = AppState::default();
    let project = ProjectService::create_project(&state, dir.path(), "Upgrade").unwrap();
    let entry = ProjectService::create_entry(
        &state,
        project.project_id,
        None,
        None,
        Some("Existing place".into()),
    )
    .unwrap();
    ProjectService::close_project(&state, project.project_id).unwrap();
    let root = std::path::Path::new(&project.package_path);
    let db = Connection::open(root.join("data").join("project.sqlite")).unwrap();
    db.execute_batch("DROP TRIGGER rich_document_owner_insert; DROP TRIGGER rich_document_owner_update; DROP TRIGGER rich_document_keep_entry; DROP TRIGGER rich_document_keep_chapter; DROP TABLE occurrence_entry; DROP TABLE occurrence_chapter; DROP TABLE temporal_occurrence; DROP TRIGGER occurrence_event_preserve; DROP TABLE timeline_calendar; DELETE FROM capability_def WHERE id='event'; DROP TRIGGER search_source_updated; DROP TRIGGER search_source_created; DROP TABLE search_index; DROP TABLE derived_index_state; DROP TABLE entry_alias; DROP TABLE story_link_role; DROP TABLE story_link; DROP TABLE story_role; DROP TABLE rich_document; DROP TABLE story_unit; DROP TRIGGER entry_materialize_capabilities; DROP TABLE spatial_node; DROP TABLE entry_capability; DROP TABLE category_capability_default; DROP TABLE type_capability_default; DROP TABLE capability_def; PRAGMA user_version=7; UPDATE project_meta SET schema_version=7; CREATE TRIGGER fail_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;").unwrap();
    let mut manifest = Manifest::read(&root.join("manifest.json")).unwrap();
    manifest.schema_version = 7;
    manifest.write(&root.join("manifest.json")).unwrap();
    assert!(ProjectService::open_project(&state, root, false).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
    assert!(db.prepare("SELECT * FROM spatial_node").is_err());
    db.execute_batch("DROP TRIGGER fail_upgrade").unwrap();
    drop(db);
    let open = ProjectService::open_project(&state, root, false).unwrap();
    assert_eq!(
        open.schema_version,
        worldcrafter_lib::persistence::migrations::CURRENT_SCHEMA_VERSION
    );
    let snapshot = ProjectService::read_spatial(&state, project.project_id).unwrap();
    assert_eq!(snapshot.entries[0].id, entry.id);
    assert!(!snapshot.entries[0].spatial);
    let db = Connection::open(root.join("data").join("project.sqlite")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT capability_id FROM entry_capability WHERE entry_id=?1",
            params![entry.id.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "base"
    );
    let recovery = Connection::open_with_flags(
        dir.path()
            .join(".worldcrafter-migration-recovery")
            .join(project.project_id.to_string())
            .join("schema-v7.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        recovery
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
    assert_eq!(
        recovery
            .query_row("SELECT authored_name FROM entry", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "Existing place"
    );
    ProjectService::close_project(&state, project.project_id).unwrap();
}
