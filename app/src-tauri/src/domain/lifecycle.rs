use super::{story::WorkspaceState, CategoryId, EntryId, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum StructureCommand {
    SetTypeRetired {
        id: TypeId,
        retired: bool,
    },
    RenameType {
        id: TypeId,
        name: String,
    },
    RenameCategory {
        id: CategoryId,
        name: String,
    },
    DeleteCategory {
        id: CategoryId,
        destination_id: CategoryId,
        remove_types: bool,
    },
    SetEntryState {
        id: EntryId,
        state: WorkspaceState,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDeletePreview {
    pub global_revision: i64,
    pub name: String,
    pub entry_count: i64,
    pub typed_entry_count: i64,
    pub type_names: Vec<String>,
    pub default_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureOutcome {
    pub global_revision: i64,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeRetirementPreview {
    pub global_revision: i64,
    pub type_id: TypeId,
    pub category_id: CategoryId,
    pub name: String,
    pub retired_at: Option<String>,
    pub assigned_entries: Vec<TypeAssignedEntry>,
    pub descendant_types: Vec<TypeDescendant>,
    pub direct_entry_count: usize,
    pub inherited_entry_count: usize,
    pub default_fields: Vec<TypeDefaultField>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeAssignedEntry {
    pub id: EntryId,
    pub label: String,
    pub workspace_state: String,
    pub type_id: TypeId,
    pub type_name: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDescendant {
    pub id: TypeId,
    pub name: String,
    pub parent_type_id: Option<TypeId>,
    pub retired_at: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefaultField {
    pub id: super::structure::FieldId,
    pub name: String,
    pub kind: String,
}
