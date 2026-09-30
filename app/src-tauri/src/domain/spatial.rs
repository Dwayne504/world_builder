//! Entry-owned features and structural containment, independent of Relationships.
use super::{CategoryId, EntryId, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityProvider {
    Category { id: CategoryId },
    Type { id: TypeId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialEntry {
    pub id: EntryId,
    pub label: String,
    pub workspace_state: String,
    pub spatial: bool,
    pub parent_id: Option<EntryId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialSnapshot {
    pub global_revision: i64,
    pub entries: Vec<SpatialEntry>,
    pub defaults: Vec<CapabilityProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SpatialCommand {
    SetEnabled {
        entry_id: EntryId,
        enabled: bool,
    },
    Reparent {
        entry_id: EntryId,
        parent_id: Option<EntryId>,
    },
    CreateChild {
        parent_id: EntryId,
        name: Option<String>,
        category_id: Option<CategoryId>,
        type_id: Option<TypeId>,
    },
    SetDefault {
        provider: CapabilityProvider,
        enabled: bool,
    },
}
