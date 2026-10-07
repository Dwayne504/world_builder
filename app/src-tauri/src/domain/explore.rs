//! Explore is a live read of canonical structure, not a stored collection of facts.
use super::{
    relationships::Perspective, structure::RelationshipDefinitionId, CategoryId, EntryId, TypeId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExploreRequest {
    pub query: String,
    pub category_id: Option<CategoryId>,
    pub type_id: Option<TypeId>,
    pub capability: Option<String>,
    pub workspace_state: String,
    pub relationship: Option<RelationshipFilter>,
    pub page: usize,
    pub page_size: usize,
}

impl Default for ExploreRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            category_id: None,
            type_id: None,
            capability: None,
            workspace_state: "active".into(),
            relationship: None,
            page: 0,
            page_size: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationshipFilter {
    pub definition_id: RelationshipDefinitionId,
    pub perspective: Perspective,
    pub other_entry_id: Option<EntryId>,
    pub include_contained: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreChoice {
    pub id: String,
    pub name: String,
    pub category_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreDefinition {
    pub id: String,
    pub name: String,
    pub forward_label: String,
    pub inverse_label: String,
    pub directed: bool,
    pub retired: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub type_name: Option<String>,
    pub workspace_state: String,
    pub spatial: bool,
    /// Direct means the chosen other Entry, contained means a primary-tree descendant.
    pub relationship_match: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResults {
    pub global_revision: i64,
    pub categories: Vec<ExploreChoice>,
    pub types: Vec<ExploreChoice>,
    pub capabilities: Vec<String>,
    pub definitions: Vec<ExploreDefinition>,
    pub selected_other: Option<ExploreEntry>,
    pub issues: Vec<String>,
    pub entries: Vec<ExploreEntry>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
