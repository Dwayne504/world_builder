//! Field definitions, template availability and authored values are distinct.
use super::relationships::{OtherEntry, Perspective, Relationship};
use super::structure::{ChoiceOptionId, FieldId};
use super::structure::{RelationshipDefinitionId, RelationshipId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    ShortText,
    RichText,
    Number,
    Boolean,
    Choice,
    MultiChoice,
    Relationship,
}

impl FieldKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShortText => "short_text",
            Self::RichText => "rich_text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Choice => "choice",
            Self::MultiChoice => "multi_choice",
            Self::Relationship => "relationship",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Category,
    Type,
    Entry,
}
impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Category => "category",
            Self::Type => "type",
            Self::Entry => "entry",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldProvider {
    pub kind: ProviderKind,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FieldValue {
    Text(String),
    RichText(RichFieldValue),
    Number(f64),
    Boolean(bool),
    Choices(Vec<ChoiceOptionId>),
}

/// Versioned document content; display metadata is derived, never trusted on save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RichFieldValue {
    pub schema_version: i64,
    pub content: Option<serde_json::Value>,
    pub revision: i64,
    #[serde(default)]
    pub plain_text: String,
    #[serde(default)]
    pub read_only_reason: Option<String>,
    #[serde(default)]
    pub original_json: Option<String>,
}

impl FieldValue {
    pub fn matches_kind(&self, kind: FieldKind) -> bool {
        match (self, kind) {
            (Self::Text(_), FieldKind::ShortText) | (Self::Boolean(_), FieldKind::Boolean) => true,
            (Self::RichText(_), FieldKind::RichText) => true,
            (Self::Number(n), FieldKind::Number) => n.is_finite(),
            (Self::Choices(ids), FieldKind::Choice) => ids.len() == 1,
            (Self::Choices(ids), FieldKind::MultiChoice) => !ids.is_empty(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldEdit {
    pub field_id: FieldId,
    pub value: Option<FieldValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum FieldCommand {
    CreateProjection {
        name: String,
        relationship_definition_id: RelationshipDefinitionId,
        perspective: Perspective,
        provider: FieldProvider,
    },
    EditProjection {
        field_id: FieldId,
        other: OtherEntry,
        instance_id: Option<RelationshipId>,
    },
    RemoveProjection {
        field_id: FieldId,
    },
    Create {
        name: String,
        field_kind: FieldKind,
        #[serde(default)]
        unit: Option<String>,
        provider: FieldProvider,
        options: Vec<String>,
        value: Option<FieldValue>,
    },
    SetHidden {
        field_id: FieldId,
        hidden: bool,
    },
    SetValues {
        edits: Vec<FieldEdit>,
    },
    Rename {
        field_id: FieldId,
        name: String,
    },
    SetRetired {
        field_id: FieldId,
        retired: bool,
    },
    Bind {
        field_id: FieldId,
        provider: FieldProvider,
    },
    Unbind {
        field_id: FieldId,
        provider: FieldProvider,
    },
    AddChoice {
        field_id: FieldId,
        label: String,
    },
    RenameChoice {
        option_id: ChoiceOptionId,
        label: String,
    },
    SetChoiceRetired {
        option_id: ChoiceOptionId,
        retired: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    pub id: ChoiceOptionId,
    pub label: String,
    pub retired: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldBinding {
    pub provider: FieldProvider,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldDefinition {
    pub id: FieldId,
    pub name: String,
    pub kind: FieldKind,
    pub unit: Option<String>,
    pub retired: bool,
    pub revision: i64,
    pub options: Vec<ChoiceOption>,
    pub bindings: Vec<FieldBinding>,
    pub projection: Option<FieldProjection>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldProjection {
    pub relationship_definition_id: RelationshipDefinitionId,
    pub perspective: Perspective,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryField {
    pub definition: FieldDefinition,
    pub available: bool,
    pub hidden: bool,
    pub default_sources: Vec<FieldBinding>,
    pub value: Option<FieldValue>,
    /// Derived from canonical current relationships, never a Field Value.
    pub projected_relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryFields {
    pub global_revision: i64,
    pub fields: Vec<EntryField>,
    pub definitions: Vec<FieldDefinition>,
}

/// Shared definitions can be managed before the first Entry exists.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldCatalog {
    pub global_revision: i64,
    pub definitions: Vec<FieldDefinition>,
}
impl From<EntryFields> for FieldCatalog {
    fn from(snapshot: EntryFields) -> Self {
        Self {
            global_revision: snapshot.global_revision,
            definitions: snapshot.definitions,
        }
    }
}

/// A reviewed, project-wide merge. Values and availability are assessed by IDs,
/// never silently coalesced because two visible names happen to match.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMergeEntry {
    pub entry_id: super::structure::EntryId,
    pub name: String,
    pub source_value: Option<FieldValue>,
    pub target_value: Option<FieldValue>,
    pub conflict: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMergePreview {
    pub global_revision: i64,
    pub source: FieldDefinition,
    pub target: FieldDefinition,
    pub entries: Vec<FieldMergeEntry>,
    pub blockers: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMergeOutcome {
    pub global_revision: i64,
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFieldDeleteOutcome {
    pub snapshot: EntryFields,
    pub backup_path: String,
}
