use rusqlite::{params, Connection};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        explore::*, relationships::*, spatial::SpatialCommand, structure::RelationshipDefinitionId,
        CategoryId, EntryId, ProjectId, TypeId,
    },
};

struct Fixture {
    _dir: TempDir,
    state: AppState,
    project: ProjectId,
    path: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let p = ProjectService::create_project(&state, dir.path(), "Explore fixture").unwrap();
        Self {
            _dir: dir,
            state,
            project: p.project_id,
            path: p.package_path,
        }
    }
    fn db(&self) -> Connection {
        Connection::open(std::path::Path::new(&self.path).join("data/project.sqlite")).unwrap()
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
    fn entry(&self, name: &str) -> EntryId {
        ProjectService::create_entry(&self.state, self.project, None, None, Some(name.into()))
            .unwrap()
            .id
    }
    fn query(&self, r: ExploreRequest) -> ExploreResults {
        ProjectService::explore_project(&self.state, self.project, r).unwrap()
    }
    fn spatial(&self, command: SpatialCommand) {
        ProjectService::apply_spatial(&self.state, self.project, self.revision(), command).unwrap();
    }
    fn place(&self, name: &str, parent: Option<EntryId>) -> EntryId {
        let id = self.entry(name);
        self.spatial(SpatialCommand::SetEnabled {
            entry_id: id,
            enabled: true,
        });
        if parent.is_some() {
            self.spatial(SpatialCommand::Reparent {
                entry_id: id,
                parent_id: parent,
            });
        }
        id
    }
    fn relation(&self, entry: EntryId, command: RelationshipCommand) -> EntryRelationships {
        ProjectService::apply_relationships(
            &self.state,
            self.project,
            entry,
            self.revision(),
            command,
        )
        .unwrap()
    }
    fn definition(&self, entry: EntryId, directed: bool) -> RelationshipDefinitionId {
        self.relation(
            entry,
            RelationshipCommand::CreateDefinition {
                draft: DefinitionDraft {
                    name: "Chosen relation".into(),
                    forward_label: "connects to".into(),
                    inverse_label: if directed {
                        "is connected from"
                    } else {
                        "connects to"
                    }
                    .into(),
                    directed,
                    expected_targets_per_source: None,
                    expected_sources_per_target: None,
                },
            },
        )
        .definitions
        .last()
        .unwrap()
        .id
    }
    fn connect(&self, a: EntryId, b: EntryId, d: RelationshipDefinitionId) -> EntryRelationships {
        self.relation(
            a,
            RelationshipCommand::Connect {
                definition_id: d,
                perspective: Perspective::Source,
                other: OtherEntry::Existing { id: b },
                note: "Authored note".into(),
                replace: vec![],
            },
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}
fn related(
    definition_id: RelationshipDefinitionId,
    other: EntryId,
    contained: bool,
) -> ExploreRequest {
    ExploreRequest {
        relationship: Some(RelationshipFilter {
            definition_id,
            perspective: Perspective::Source,
            other_entry_id: Some(other),
            include_contained: contained,
        }),
        ..Default::default()
    }
}
fn ids(result: &ExploreResults) -> Vec<String> {
    result.entries.iter().map(|e| e.id.clone()).collect()
}

#[test]
fn category_exact_type_capability_and_unicode_alias_compose_without_inheriting_types() {
    let f = Fixture::new();
    let category = ProjectService::create_category(&f.state, f.project, "People")
        .unwrap()
        .id;
    let parent = ProjectService::create_type(&f.state, f.project, category, None, "Human")
        .unwrap()
        .id;
    let child =
        ProjectService::create_type(&f.state, f.project, category, Some(parent), "Specialist")
            .unwrap()
            .id;
    let a = ProjectService::create_entry(
        &f.state,
        f.project,
        Some(category),
        Some(parent),
        Some("Captain".into()),
    )
    .unwrap()
    .id;
    let b = ProjectService::create_entry(
        &f.state,
        f.project,
        Some(category),
        Some(child),
        Some("Captain junior".into()),
    )
    .unwrap()
    .id;
    f.spatial(SpatialCommand::SetEnabled {
        entry_id: a,
        enabled: true,
    });
    ProjectService::apply_alias(
        &f.state,
        f.project,
        a,
        f.revision(),
        worldcrafter_lib::domain::search::AliasCommand::Add {
            text: "Léopold".into(),
        },
    )
    .unwrap();
    let mut request = ExploreRequest {
        query: "LEOPOLD".into(),
        category_id: Some(category),
        type_id: Some(parent),
        capability: Some("spatial".into()),
        ..Default::default()
    };
    assert_eq!(ids(&f.query(request.clone())), vec![a.to_string()]);
    let other = f.entry("Target");
    let definition = f.definition(a, true);
    f.connect(a, other, definition);
    request.relationship = related(definition, other, false).relationship;
    assert_eq!(ids(&f.query(request.clone())), vec![a.to_string()]);
    request.relationship = None;
    request.query.clear();
    request.capability = None;
    assert_eq!(ids(&f.query(request.clone())), vec![a.to_string()]);
    request.type_id = Some(child);
    assert_eq!(ids(&f.query(request.clone())), vec![b.to_string()]);
    request.capability = Some("event".into());
    assert_eq!(f.query(request).total, 0);
}

#[test]
fn contained_targets_follow_moves_and_inactive_ancestors_without_rewriting_direct_relationships() {
    let f = Fixture::new();
    let root = f.place("Island", None);
    let city = f.place("City", Some(root));
    let room = f.place("Room", Some(city));
    let outside = f.place("Elsewhere", None);
    let person = f.entry("Person");
    let direct = f.entry("Direct");
    let d = f.definition(person, true);
    f.connect(person, room, d);
    f.connect(direct, root, d);
    let exact = f.query(related(d, root, false));
    assert_eq!(ids(&exact), vec![direct.to_string()]);
    assert_eq!(
        exact.entries[0].relationship_match.as_deref(),
        Some("direct")
    );
    let mut request = related(d, root, true);
    let result = f.query(request.clone());
    assert_eq!(result.total, 2);
    assert_eq!(
        result
            .entries
            .iter()
            .find(|e| e.id == person.to_string())
            .unwrap()
            .relationship_match
            .as_deref(),
        Some("contained")
    );
    f.db()
        .execute(
            "UPDATE record_identity SET workspace_state='archived' WHERE record_id=?1",
            [city.to_string()],
        )
        .unwrap();
    assert_eq!(f.query(request.clone()).total, 2);
    f.db()
        .execute(
            "UPDATE record_identity SET workspace_state='active' WHERE record_id=?1",
            [city.to_string()],
        )
        .unwrap();
    f.spatial(SpatialCommand::Reparent {
        entry_id: city,
        parent_id: Some(outside),
    });
    assert_eq!(ids(&f.query(request.clone())), vec![direct.to_string()]);
    request.relationship.as_mut().unwrap().other_entry_id = Some(outside);
    assert_eq!(ids(&f.query(request)), vec![person.to_string()]);
    assert_eq!(f.db().query_row("SELECT record_id FROM relationship_participant WHERE slot='target' AND record_id=?1",[room.to_string()],|r|r.get::<_,String>(0)).unwrap(),room.to_string());
}

#[test]
fn direction_current_state_and_symmetric_relations_are_explicit() {
    let f = Fixture::new();
    let a = f.entry("A");
    let b = f.entry("B");
    let d = f.definition(a, true);
    let link = f.connect(a, b, d).relationships[0].id;
    assert_eq!(ids(&f.query(related(d, b, false))), vec![a.to_string()]);
    assert_eq!(f.query(related(d, a, false)).total, 0);
    let mut inverse = related(d, a, false);
    inverse.relationship.as_mut().unwrap().perspective = Perspective::Target;
    assert_eq!(ids(&f.query(inverse.clone())), vec![b.to_string()]);
    f.relation(
        a,
        RelationshipCommand::SetEnded {
            id: link,
            ended: true,
        },
    );
    assert_eq!(f.query(inverse).total, 0);
    let symmetric = f.definition(a, false);
    f.connect(a, b, symmetric);
    assert_eq!(
        ids(&f.query(related(symmetric, a, false))),
        vec![b.to_string()]
    );
    assert_eq!(
        ids(&f.query(related(symmetric, b, false))),
        vec![a.to_string()]
    );
    f.db().execute("UPDATE record_identity SET workspace_state='archived' WHERE kind='relationship_instance'",[]).unwrap();
    assert_eq!(f.query(related(symmetric, b, false)).total, 0);
}

#[test]
fn unresolved_selections_never_widen_and_invalid_requests_fail() {
    let f = Fixture::new();
    let a = f.entry("A");
    let d = f.definition(a, true);
    for request in [
        ExploreRequest {
            category_id: Some(CategoryId::new()),
            ..Default::default()
        },
        ExploreRequest {
            type_id: Some(TypeId::new()),
            ..Default::default()
        },
        ExploreRequest {
            capability: Some("invented".into()),
            ..Default::default()
        },
        related(RelationshipDefinitionId::new(), a, false),
        related(d, EntryId::new(), false),
        related(d, a, true),
    ] {
        let result = f.query(request);
        assert!(!result.issues.is_empty());
        assert_eq!(result.total, 0);
        assert!(result.entries.is_empty());
    }
    let c = ProjectService::create_category(&f.state, f.project, "Other")
        .unwrap()
        .id;
    let t = ProjectService::create_type(&f.state, f.project, c, None, "T")
        .unwrap()
        .id;
    let wrong_category = f.query(ExploreRequest {
        category_id: Some(
            ProjectService::list_categories(&f.state, f.project)
                .unwrap()
                .into_iter()
                .find(|v| v.is_uncategorized)
                .unwrap()
                .id,
        ),
        type_id: Some(t),
        ..Default::default()
    });
    assert!(!wrong_category.issues.is_empty());
    f.relation(
        a,
        RelationshipCommand::RetireDefinition {
            definition_id: d,
            retired: true,
        },
    );
    assert!(!f.query(related(d, a, false)).issues.is_empty());
    for request in [
        ExploreRequest {
            page_size: 0,
            ..Default::default()
        },
        ExploreRequest {
            page_size: 101,
            ..Default::default()
        },
        ExploreRequest {
            page: usize::MAX,
            ..Default::default()
        },
        ExploreRequest {
            query: "x".repeat(201),
            ..Default::default()
        },
        ExploreRequest {
            workspace_state: "invented".into(),
            ..Default::default()
        },
    ] {
        assert!(ProjectService::explore_project(&f.state, f.project, request).is_err());
    }
    assert!(
        ProjectService::explore_project(&f.state, ProjectId::new(), ExploreRequest::default())
            .is_err()
    );
}

#[test]
fn stable_filters_read_current_names_and_states_without_mutating_sources() {
    let f = Fixture::new();
    let a = f.entry("A");
    let b = f.entry("B");
    let d = f.definition(a, true);
    f.connect(a, b, d);
    let request = related(d, b, false);
    let revision = f.revision();
    f.db()
        .execute(
            "UPDATE entry SET authored_name='Renamed target' WHERE id=?1",
            [b.to_string()],
        )
        .unwrap();
    f.db()
        .execute(
            "UPDATE relationship_definition SET name='Renamed definition' WHERE id=?1",
            [d.to_string()],
        )
        .unwrap();
    let result = f.query(request.clone());
    assert_eq!(ids(&result), vec![a.to_string()]);
    assert_eq!(result.selected_other.unwrap().name, "Renamed target");
    assert_eq!(result.definitions[0].name, "Renamed definition");
    assert_eq!(f.revision(), revision);
    f.db()
        .execute(
            "UPDATE record_identity SET workspace_state='trashed' WHERE record_id=?1",
            [a.to_string()],
        )
        .unwrap();
    assert_eq!(f.query(request.clone()).total, 0);
    let mut inactive = request;
    inactive.workspace_state = "trashed".into();
    assert_eq!(ids(&f.query(inactive)), vec![a.to_string()]);
    assert_eq!(
        f.db()
            .query_row("SELECT note FROM relationship_instance", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "Authored note"
    );
}

#[test]
fn large_results_are_bounded_deterministic_and_recomputed_after_reopen() {
    let f = Fixture::new();
    for i in 0..1007 {
        f.entry(&format!("Entry {:03}", i / 2));
    }
    let revision = f.revision();
    let mut all = vec![];
    for page in 0..51 {
        let result = f.query(ExploreRequest {
            page,
            ..Default::default()
        });
        assert_eq!(result.total, 1007);
        assert!(result.entries.len() <= 20);
        all.extend(ids(&result));
    }
    all.sort();
    all.dedup();
    assert_eq!(all.len(), 1007);
    assert_eq!(f.revision(), revision);
    ProjectService::close_project(&f.state, f.project).unwrap();
    assert!(
        ProjectService::explore_project(&f.state, f.project, ExploreRequest::default()).is_err()
    );
    ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
    assert_eq!(f.query(Default::default()).total, 1007);
    let count: i64 = f
        .db()
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name LIKE '%explore%'",
            params![],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}
