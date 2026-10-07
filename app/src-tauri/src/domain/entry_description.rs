//! Optional Entry prose, independent from structured Fields and Chapter writing.
use super::EntryId;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDescriptionDocument {
    pub id: String,
    pub schema_version: i64,
    pub content: Option<Value>,
    pub plain_text: String,
    pub word_count: usize,
    pub revision: i64,
    pub read_only_reason: Option<String>,
    pub original_json: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDescriptionSnapshot {
    pub global_revision: i64,
    pub entry_id: EntryId,
    pub workspace_state: String,
    pub document: Option<EntryDescriptionDocument>,
}
