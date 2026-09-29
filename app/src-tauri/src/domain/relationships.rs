//! Semantic connections own identities and participants, never duplicate Field values.
use super::{
    structure::{RelationshipDefinitionId, RelationshipId},
    CategoryId, EntryId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionDraft {
    pub name: String,
    pub forward_label: String,
    pub inverse_label: String,
    pub directed: bool,
    pub expected_targets_per_source: Option<u32>,
    pub expected_sources_per_target: Option<u32>,
}
impl DefinitionDraft {
    pub fn validated(mut self) -> Result<Self, String> {
        self.name = super::require_definition_name(&self.name).map_err(|e| e.to_string())?;
        self.forward_label =
            super::require_definition_name(&self.forward_label).map_err(|e| e.to_string())?;
        self.inverse_label =
            super::require_definition_name(&self.inverse_label).map_err(|e| e.to_string())?;
        if self.expected_targets_per_source == Some(0)
            || self.expected_sources_per_target == Some(0)
        {
            return Err("An expectation must be a positive number or left empty".into());
        }
        if !self.directed
            && (self.forward_label != self.inverse_label
                || self.expected_targets_per_source != self.expected_sources_per_target)
        {
            return Err(
                "A symmetric relationship uses the same label and expectation on both sides".into(),
            );
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDefinition {
    pub id: RelationshipDefinitionId,
    #[serde(flatten)]
    pub draft: DefinitionDraft,
    pub retired: bool,
    pub revision: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perspective {
    Source,
    Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum OtherEntry {
    Existing {
        id: EntryId,
    },
    Create {
        name: Option<String>,
        category_id: Option<CategoryId>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum RelationshipCommand {
    CreateDefinition {
        draft: DefinitionDraft,
    },
    UpdateDefinition {
        definition_id: RelationshipDefinitionId,
        draft: DefinitionDraft,
    },
    RetireDefinition {
        definition_id: RelationshipDefinitionId,
        retired: bool,
    },
    Connect {
        definition_id: RelationshipDefinitionId,
        perspective: Perspective,
        other: OtherEntry,
        note: String,
        replace: Vec<RelationshipId>,
    },
    SetNote {
        id: RelationshipId,
        note: String,
    },
    SetEnded {
        id: RelationshipId,
        ended: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub id: Option<EntryId>,
    pub label: String,
    pub workspace_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub id: RelationshipId,
    pub definition_id: RelationshipDefinitionId,
    pub source: Participant,
    pub target: Participant,
    pub note: String,
    pub ended: bool,
    pub workspace_state: String,
    pub revision: i64,
    pub warnings: Vec<String>,
}
impl Relationship {
    pub fn is_current(&self) -> bool {
        !self.ended && self.workspace_state == "active"
    }
    pub fn involves(&self, entry: EntryId) -> bool {
        self.source.id == Some(entry) || self.target.id == Some(entry)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipEntry {
    pub id: EntryId,
    pub label: String,
    pub category_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryRelationships {
    pub global_revision: i64,
    pub definitions: Vec<RelationshipDefinition>,
    pub relationships: Vec<Relationship>,
    pub entries: Vec<RelationshipEntry>,
}
