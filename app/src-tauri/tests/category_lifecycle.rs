use rusqlite::Connection;
use std::path::Path;
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        fields::*, lifecycle::*, relationships::*, spatial::*, story::*, timeline::*, CategoryId,
        EntryId, ProjectId,
    },
};

struct Fixture {
    state: AppState,
    dir: TempDir,
    project: ProjectId,
    path: String,
    category: CategoryId,
    fallback: CategoryId,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let p = ProjectService::create_project(&state, dir.path(), "Lifecycle tests").unwrap();
        let fallback = ProjectService::list_categories(&state, p.project_id).unwrap()[0].id;
        let category = ProjectService::create_category(&state, p.project_id, "People")
            .unwrap()
            .id;
        Self {
            state,
            dir,
            project: p.project_id,
            path: p.package_path,
            category,
            fallback,
        }
    }
    fn revision(&self) -> i64 {
        ProjectService::read_field_catalog(&self.state, self.project)
            .unwrap()
            .global_revision
    }
    fn entry(&self, name: &str) -> EntryId {
        ProjectService::create_entry(
            &self.state,
            self.project,
            Some(self.category),
            None,
            Some(name.into()),
        )
        .unwrap()
        .id
    }
    fn apply(&self, command: StructureCommand) -> StructureOutcome {
        ProjectService::apply_structure(
            &self.state,
            self.project,
            self.revision(),
            command,
            Some(&self.dir.path().join("backups")),
        )
        .unwrap()
    }
    fn deletion(&self) -> StructureCommand {
        StructureCommand::DeleteCategory {
            id: self.category,
            destination_id: self.fallback,
            remove_types: true,
        }
    }
    fn db(&self) -> Connection {
        let db = Connection::open(Path::new(&self.path).join("data/project.sqlite")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db
    }
    fn spatial(&self, command: SpatialCommand) {
        ProjectService::apply_spatial(&self.state, self.project, self.revision(), command).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}

#[test]
fn category_delete_moves_all_lifecycle_states_and_keeps_values_capabilities_and_recovery() {
    let f = Fixture::new();
    let parent =
        ProjectService::create_type(&f.state, f.project, f.category, None, "Person").unwrap();
    let child =
        ProjectService::create_type(&f.state, f.project, f.category, Some(parent.id), "Human")
            .unwrap();
    f.spatial(SpatialCommand::SetDefault {
        provider: CapabilityProvider::Category { id: f.category },
        enabled: true,
    });
    let field = ProjectService::apply_template_fields(
        &f.state,
        f.project,
        f.revision(),
        FieldCommand::Create {
            name: "Age".into(),
            field_kind: FieldKind::Number,
            unit: Some("years".into()),
            provider: FieldProvider {
                kind: ProviderKind::Type,
                id: parent.id.to_string(),
            },
            options: vec![],
            value: None,
        },
    )
    .unwrap()
    .definitions[0]
        .id;
    let mut ids = vec![];
    for i in 0..3 {
        let e = ProjectService::create_entry(
            &f.state,
            f.project,
            Some(f.category),
            Some(child.id),
            Some(format!("Person {i}")),
        )
        .unwrap();
        ProjectService::apply_fields(
            &f.state,
            f.project,
            e.id,
            f.revision(),
            FieldCommand::SetValues {
                edits: vec![FieldEdit {
                    field_id: field,
                    value: Some(FieldValue::Number(48.0)),
                }],
            },
        )
        .unwrap();
        ids.push(e.id);
    }
    f.spatial(SpatialCommand::Reparent {
        entry_id: ids[1],
        parent_id: Some(ids[0]),
    });
    for (id, state) in ids.iter().zip([
        WorkspaceState::Active,
        WorkspaceState::Archived,
        WorkspaceState::Trashed,
    ]) {
        f.apply(StructureCommand::SetEntryState { id: *id, state });
    }
    let review = ProjectService::preview_category_delete(&f.state, f.project, f.category).unwrap();
    assert_eq!(
        (
            review.entry_count,
            review.typed_entry_count,
            review.default_count
        ),
        (3, 3, 1)
    );
    assert_eq!(review.type_names, vec!["Human", "Person"]);
    let outcome = f.apply(f.deletion());
    assert_eq!(outcome.global_revision, review.global_revision + 1);
    assert!(!ProjectService::list_categories(&f.state, f.project)
        .unwrap()
        .iter()
        .any(|c| c.id == f.category));
    assert!(ProjectService::list_types(&f.state, f.project, f.category)
        .unwrap()
        .is_empty());
    for (i, id) in ids.iter().enumerate() {
        let e = ProjectService::get_entry(&f.state, f.project, *id).unwrap();
        assert_eq!(e.category_id, f.fallback);
        assert_eq!(e.type_id, None);
        assert_eq!(e.workspace_state, ["active", "archived", "trashed"][i]);
        let fields = ProjectService::read_fields(&f.state, f.project, *id).unwrap();
        assert_eq!(fields.fields[0].value, Some(FieldValue::Number(48.0)));
    }
    let spatial = ProjectService::read_spatial(&f.state, f.project).unwrap();
    assert!(spatial.entries.iter().all(|e| e.spatial));
    assert_eq!(
        spatial
            .entries
            .iter()
            .find(|e| e.id == ids[1])
            .unwrap()
            .parent_id,
        Some(ids[0])
    );
    let catalog = ProjectService::read_field_catalog(&f.state, f.project).unwrap();
    assert_eq!(catalog.definitions[0].id, field);
    assert!(catalog.definitions[0].bindings.is_empty());
    let restored = ProjectService::restore_backup_as_copy(
        &f.state,
        Path::new(outcome.backup_path.as_ref().unwrap()),
        f.dir.path(),
        Some("Before deletion"),
    )
    .unwrap();
    let e = ProjectService::get_entry(&f.state, restored.project_id, ids[0]).unwrap();
    assert_eq!(e.category_id, f.category);
    assert_eq!(e.type_id, Some(child.id));
    ProjectService::close_project(&f.state, restored.project_id).unwrap();
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, ids[2])
            .unwrap()
            .category_id,
        f.fallback
    );
}

#[test]
fn category_rejects_unreviewed_stale_invalid_or_unbacked_deletion() {
    let f = Fixture::new();
    let id = f.entry("Keep me");
    ProjectService::create_type(&f.state, f.project, f.category, None, "Human").unwrap();
    let rev = f.revision();
    for cmd in [
        StructureCommand::DeleteCategory {
            id: f.category,
            destination_id: f.fallback,
            remove_types: false,
        },
        StructureCommand::DeleteCategory {
            id: f.category,
            destination_id: f.category,
            remove_types: true,
        },
        StructureCommand::DeleteCategory {
            id: f.category,
            destination_id: CategoryId::new(),
            remove_types: true,
        },
        StructureCommand::DeleteCategory {
            id: f.fallback,
            destination_id: f.category,
            remove_types: true,
        },
    ] {
        assert!(
            ProjectService::apply_structure(&f.state, f.project, rev, cmd, Some(f.dir.path()))
                .is_err()
        );
    }
    assert!(ProjectService::apply_structure(
        &f.state,
        f.project,
        rev - 1,
        f.deletion(),
        Some(f.dir.path())
    )
    .is_err());
    assert!(ProjectService::apply_structure(&f.state, f.project, rev, f.deletion(), None).is_err());
    let bad = f.dir.path().join("file-not-directory");
    std::fs::write(&bad, "blocked").unwrap();
    assert!(
        ProjectService::apply_structure(&f.state, f.project, rev, f.deletion(), Some(&bad))
            .is_err()
    );
    assert_eq!(f.revision(), rev);
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, id)
            .unwrap()
            .category_id,
        f.category
    );
}

#[test]
fn failed_category_publication_rolls_back_reassignment_and_defaults() {
    let f = Fixture::new();
    let id = f.entry("Original");
    f.spatial(SpatialCommand::SetDefault {
        provider: CapabilityProvider::Category { id: f.category },
        enabled: true,
    });
    let rev = f.revision();
    f.db().execute_batch("CREATE TRIGGER reject_category_delete BEFORE DELETE ON category BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(ProjectService::apply_structure(
        &f.state,
        f.project,
        rev,
        f.deletion(),
        Some(&f.dir.path().join("backups"))
    )
    .is_err());
    assert_eq!(f.revision(), rev);
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, id)
            .unwrap()
            .category_id,
        f.category
    );
    assert_eq!(
        ProjectService::read_spatial(&f.state, f.project)
            .unwrap()
            .defaults
            .len(),
        1
    );
}

#[test]
fn trash_is_reversible_without_cascading_to_children_or_breaking_story_timeline_and_relationships()
{
    let f = Fixture::new();
    let id = f.entry("Parent");
    let child = f.entry("Child");
    f.spatial(SpatialCommand::SetEnabled {
        entry_id: id,
        enabled: true,
    });
    f.spatial(SpatialCommand::SetEnabled {
        entry_id: child,
        enabled: true,
    });
    f.spatial(SpatialCommand::Reparent {
        entry_id: child,
        parent_id: Some(id),
    });
    let chapter = ProjectService::apply_story(
        &f.state,
        f.project,
        f.revision(),
        StoryCommand::Create {
            title: "Chapter".into(),
        },
    )
    .unwrap()
    .chapter
    .id;
    ProjectService::apply_story(
        &f.state,
        f.project,
        f.revision(),
        StoryCommand::SetLink {
            chapter_id: chapter,
            entry_id: id,
            role_ids: vec![],
        },
    )
    .unwrap();
    let occurrence =
        ProjectService::apply_timeline(&f.state, f.project, f.revision(), TimelineCommand::Create)
            .unwrap()
            .occurrences[0]
            .id;
    ProjectService::apply_timeline(
        &f.state,
        f.project,
        f.revision(),
        TimelineCommand::Save {
            id: occurrence,
            draft: OccurrenceDraft {
                title: "Event".into(),
                notes: "Keep this".into(),
                date: None,
                event_entry_id: Some(id),
                entry_ids: vec![child],
                chapter_ids: vec![chapter],
            },
        },
    )
    .unwrap();
    let definition = ProjectService::apply_relationships(
        &f.state,
        f.project,
        id,
        f.revision(),
        RelationshipCommand::CreateDefinition {
            draft: DefinitionDraft {
                name: "Kin".into(),
                forward_label: "knows".into(),
                inverse_label: "knows".into(),
                directed: false,
                expected_targets_per_source: None,
                expected_sources_per_target: None,
            },
        },
    )
    .unwrap()
    .definitions[0]
        .id;
    ProjectService::apply_relationships(
        &f.state,
        f.project,
        id,
        f.revision(),
        RelationshipCommand::Connect {
            definition_id: definition,
            perspective: Perspective::Source,
            other: OtherEntry::Existing { id: child },
            note: "Family".into(),
            replace: vec![],
        },
    )
    .unwrap();
    let rev = f.revision();
    f.apply(StructureCommand::SetEntryState {
        id,
        state: WorkspaceState::Trashed,
    });
    assert!(ProjectService::apply_structure(
        &f.state,
        f.project,
        rev,
        StructureCommand::SetEntryState {
            id,
            state: WorkspaceState::Active
        },
        None
    )
    .is_err());
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, id)
            .unwrap()
            .workspace_state,
        "trashed"
    );
    let spatial = ProjectService::read_spatial(&f.state, f.project).unwrap();
    let c = spatial.entries.iter().find(|e| e.id == child).unwrap();
    assert_eq!(c.parent_id, Some(id));
    assert_eq!(c.workspace_state, "active");
    assert_eq!(
        ProjectService::read_timeline(&f.state, f.project)
            .unwrap()
            .occurrences[0]
            .event_entry
            .as_ref()
            .unwrap()
            .workspace_state,
        "trashed"
    );
    assert_eq!(
        ProjectService::read_relationships(&f.state, f.project, id)
            .unwrap()
            .relationships[0]
            .note,
        "Family"
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM story_link", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, id)
            .unwrap()
            .workspace_state,
        "trashed"
    );
    f.apply(StructureCommand::SetEntryState {
        id,
        state: WorkspaceState::Active,
    });
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, id)
            .unwrap()
            .workspace_state,
        "active"
    );
    assert_eq!(
        ProjectService::read_relationships(&f.state, f.project, id)
            .unwrap()
            .relationships
            .len(),
        1
    );
}

#[test]
fn category_rename_keeps_identity_and_rejects_empty_names_and_fallback() {
    let f = Fixture::new();
    let id = f.entry("Someone");
    let rev = f.revision();
    for (category, name) in [(f.category, "  "), (f.fallback, "Other")] {
        assert!(ProjectService::apply_structure(
            &f.state,
            f.project,
            rev,
            StructureCommand::RenameCategory {
                id: category,
                name: name.into()
            },
            None
        )
        .is_err());
    }
    f.apply(StructureCommand::RenameCategory {
        id: f.category,
        name: "  Characters  ".into(),
    });
    let categories = ProjectService::list_categories(&f.state, f.project).unwrap();
    assert_eq!(
        categories.iter().find(|c| c.id == f.category).unwrap().name,
        "Characters"
    );
    assert_eq!(
        ProjectService::get_entry(&f.state, f.project, id)
            .unwrap()
            .category_id,
        f.category
    );
}
