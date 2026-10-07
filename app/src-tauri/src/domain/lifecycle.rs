use super::{story::WorkspaceState, CategoryId, EntryId, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum StructureCommand {
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
