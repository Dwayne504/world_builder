//! Field definitions, template availability and authored values are distinct.
use super::structure::{ChoiceOptionId, FieldId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    ShortText,
    Number,
    Boolean,
    Choice,
    MultiChoice,
}

impl FieldKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShortText => "short_text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Choice => "choice",
            Self::MultiChoice => "multi_choice",
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
    Number(f64),
    Boolean(bool),
    Choices(Vec<ChoiceOptionId>),
}

impl FieldValue {
    pub fn matches_kind(&self, kind: FieldKind) -> bool {
        match (self, kind) {
            (Self::Text(_), FieldKind::ShortText) | (Self::Boolean(_), FieldKind::Boolean) => true,
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
    Create {
        name: String,
        field_kind: FieldKind,
        #[serde(default)]
        unit: Option<String>,
        provider: FieldProvider,
        options: Vec<String>,
        value: Option<FieldValue>,
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
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryField {
    pub definition: FieldDefinition,
    pub available: bool,
    pub value: Option<FieldValue>,
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
