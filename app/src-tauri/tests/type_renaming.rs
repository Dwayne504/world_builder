use std::path::Path;
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{AppState, ProjectService},
    domain::{
        fields::*, lifecycle::*, relationships::*, search::*, spatial::*, story::*, CategoryId,
        ProjectId, TypeId,
    },
};

struct Fixture {
    state: AppState,
    dir: TempDir,
    project: ProjectId,
    path: String,
    category: CategoryId,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let project = ProjectService::create_project(&state, dir.path(), "Type tests").unwrap();
        let category = ProjectService::create_category(&state, project.project_id, "People")
            .unwrap()
            .id;
        Self {
            state,
            dir,
            project: project.project_id,
            path: project.package_path,
            category,
        }
    }
    fn revision(&self) -> i64 {
        ProjectService::read_field_catalog(&self.state, self.project)
            .unwrap()
            .global_revision
    }
    fn rename(&self, id: TypeId, name: &str) -> StructureOutcome {
        ProjectService::apply_structure(
            &self.state,
            self.project,
            self.revision(),
            StructureCommand::RenameType {
                id,
                name: name.into(),
            },
            None,
        )
        .unwrap()
    }
    fn search(&self) -> SearchResults {
        ProjectService::search_project(
            &self.state,
            self.project,
            SearchRequest {
                query: "Captain".into(),
                include_inactive: true,
                limit_per_group: 10,
                entry_id: None,
                structured_kind: None,
                text_area: None,
                story_role_id: None,
            },
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project);
    }
}

#[test]
fn rename_preserves_identity_inheritance_values_links_and_search_through_reopen_and_restore() {
    let f = Fixture::new();
    let parent =
        ProjectService::create_type(&f.state, f.project, f.category, None, "Person").unwrap();
    let child =
        ProjectService::create_type(&f.state, f.project, f.category, Some(parent.id), "Human")
            .unwrap();
    ProjectService::apply_spatial(
        &f.state,
        f.project,
        f.revision(),
        SpatialCommand::SetDefault {
            provider: CapabilityProvider::Type { id: parent.id },
            enabled: true,
        },
    )
    .unwrap();
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
    let entry = ProjectService::create_entry(
        &f.state,
        f.project,
        Some(f.category),
        Some(child.id),
        Some("Captain".into()),
    )
    .unwrap();
    let other = ProjectService::create_entry(
        &f.state,
        f.project,
        Some(f.category),
        None,
        Some("Friend".into()),
    )
    .unwrap();
    ProjectService::apply_fields(
        &f.state,
        f.project,
        entry.id,
        f.revision(),
        FieldCommand::SetValues {
            edits: vec![FieldEdit {
                field_id: field,
                value: Some(FieldValue::Number(48.0)),
            }],
        },
    )
    .unwrap();
    let chapter = ProjectService::apply_story(
        &f.state,
        f.project,
        f.revision(),
        StoryCommand::Create {
            title: "Voyage".into(),
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
            entry_id: entry.id,
            role_ids: vec![],
        },
    )
    .unwrap();
    let definition = ProjectService::apply_relationships(
        &f.state,
        f.project,
        entry.id,
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
    let relationships = ProjectService::apply_relationships(
        &f.state,
        f.project,
        entry.id,
        f.revision(),
        RelationshipCommand::Connect {
            definition_id: definition,
            perspective: Perspective::Source,
            other: OtherEntry::Existing { id: other.id },
            note: "Keep this note".into(),
            replace: vec![],
        },
    )
    .unwrap()
    .relationships;
    let links = ProjectService::read_chapter(&f.state, f.project, chapter)
        .unwrap()
        .links;
    let before = f.search(); // Materialize the old derived search index.
    assert!(before
        .groups
        .iter()
        .flat_map(|g| &g.hits)
        .any(|hit| hit.context == "People · Human"));
    let rev = f.revision();
    let result = f.rename(child.id, "  Astronaut  ");
    assert_eq!(result.global_revision, rev + 1);
    assert!(result.backup_path.is_none());
    f.rename(parent.id, "Sentient");
    let types = ProjectService::list_types(&f.state, f.project, f.category).unwrap();
    let renamed = types.iter().find(|t| t.id == child.id).unwrap();
    assert_eq!(renamed.name, "Astronaut");
    assert_eq!(renamed.parent_type_id, Some(parent.id));
    assert_eq!(renamed.category_id, f.category);
    assert_eq!(renamed.revision, child.revision + 1);
    let unchanged_entry = ProjectService::get_entry(&f.state, f.project, entry.id).unwrap();
    assert_eq!(unchanged_entry.type_id, Some(child.id));
    assert_eq!(unchanged_entry.revision, entry.revision);
    let fields = ProjectService::read_fields(&f.state, f.project, entry.id).unwrap();
    assert_eq!(fields.fields[0].value, Some(FieldValue::Number(48.0)));
    assert_eq!(
        fields.definitions[0].bindings[0].provider.id,
        parent.id.to_string()
    );
    assert!(fields.definitions[0].bindings[0].label.contains("Sentient"));
    assert_eq!(
        serde_json::to_value(
            ProjectService::read_chapter(&f.state, f.project, chapter)
                .unwrap()
                .links
        )
        .unwrap(),
        serde_json::to_value(links).unwrap()
    );
    assert_eq!(
        serde_json::to_value(
            ProjectService::read_relationships(&f.state, f.project, entry.id)
                .unwrap()
                .relationships
        )
        .unwrap(),
        serde_json::to_value(relationships).unwrap()
    );
    let after = f.search();
    assert!(after
        .groups
        .iter()
        .flat_map(|g| &g.hits)
        .any(|hit| hit.context == "People · Astronaut"));
    assert!(!after
        .groups
        .iter()
        .flat_map(|g| &g.hits)
        .any(|hit| hit.context.contains("Human")));
    let new_entry =
        ProjectService::create_entry(&f.state, f.project, Some(f.category), Some(child.id), None)
            .unwrap();
    assert!(ProjectService::read_spatial(&f.state, f.project)
        .unwrap()
        .entries
        .iter()
        .any(|e| e.id == new_entry.id && e.spatial));
    assert_eq!(
        ProjectService::read_fields(&f.state, f.project, new_entry.id)
            .unwrap()
            .fields[0]
            .definition
            .id,
        field
    );
    let backup =
        ProjectService::create_backup(&f.state, f.project, &f.dir.path().join("backups")).unwrap();
    let restored = ProjectService::restore_backup_as_copy(
        &f.state,
        Path::new(&backup),
        f.dir.path(),
        Some("Renamed copy"),
    )
    .unwrap();
    assert_ne!(restored.project_id, f.project);
    assert_eq!(
        ProjectService::get_entry(&f.state, restored.project_id, entry.id)
            .unwrap()
            .type_id,
        Some(child.id)
    );
    assert_eq!(
        ProjectService::list_types(&f.state, restored.project_id, f.category)
            .unwrap()
            .iter()
            .find(|t| t.id == child.id)
            .unwrap()
            .name,
        "Astronaut"
    );
    ProjectService::close_project(&f.state, restored.project_id).unwrap();
    ProjectService::close_project(&f.state, f.project).unwrap();
    ProjectService::open_project(&f.state, Path::new(&f.path), false).unwrap();
    assert_eq!(
        ProjectService::list_types(&f.state, f.project, f.category)
            .unwrap()
            .iter()
            .find(|t| t.id == child.id)
            .unwrap()
            .name,
        "Astronaut"
    );
}

#[test]
fn invalid_stale_missing_and_wrong_project_renames_leave_the_project_unchanged() {
    let f = Fixture::new();
    let kind = ProjectService::create_type(&f.state, f.project, f.category, None, "Human").unwrap();
    let rev = f.revision();
    for (id, name, expected, project) in [
        (kind.id, " \t\n".into(), rev, f.project),
        (kind.id, "x".repeat(201), rev, f.project),
        (kind.id, "Stale".into(), rev - 1, f.project),
        (TypeId::new(), "Missing".into(), rev, f.project),
        (kind.id, "Wrong Project".into(), rev, ProjectId::new()),
    ] {
        assert!(ProjectService::apply_structure(
            &f.state,
            project,
            expected,
            StructureCommand::RenameType { id, name },
            None
        )
        .is_err());
        assert_eq!(f.revision(), rev);
        assert_eq!(
            ProjectService::list_types(&f.state, f.project, f.category).unwrap()[0].name,
            "Human"
        );
    }
    let other = ProjectService::create_project(&f.state, f.dir.path(), "Other Project").unwrap();
    assert!(ProjectService::apply_structure(
        &f.state,
        other.project_id,
        other.revision,
        StructureCommand::RenameType {
            id: kind.id,
            name: "Cross Project".into()
        },
        None
    )
    .is_err());
    ProjectService::close_project(&f.state, other.project_id).unwrap();
    assert_eq!(f.revision(), rev);
}

#[test]
fn duplicate_names_and_unicode_rename_only_the_selected_stable_id() {
    let f = Fixture::new();
    let first =
        ProjectService::create_type(&f.state, f.project, f.category, None, "Human").unwrap();
    let second =
        ProjectService::create_type(&f.state, f.project, f.category, None, "Human").unwrap();
    f.rename(second.id, "人类 · Étoile");
    let types = ProjectService::list_types(&f.state, f.project, f.category).unwrap();
    assert_eq!(
        types.iter().find(|t| t.id == first.id).unwrap().name,
        "Human"
    );
    assert_eq!(
        types.iter().find(|t| t.id == second.id).unwrap().name,
        "人类 · Étoile"
    );
    f.rename(second.id, "Human");
    assert_eq!(
        ProjectService::list_types(&f.state, f.project, f.category)
            .unwrap()
            .len(),
        2
    );
}
