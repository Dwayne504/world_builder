//! Search is navigation over saved sources, never a source of world facts.
use super::EntryId;
use serde::{Deserialize, Serialize};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub fn normalize_alias(text: &str) -> String {
    text.nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn normalize_search(text: &str) -> String {
    text.nfkd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn tokens(text: &str) -> Vec<String> {
    normalize_search(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchRequest {
    pub query: String,
    pub include_inactive: bool,
    pub limit_per_group: usize,
    #[serde(default)]
    pub entry_id: Option<EntryId>,
    #[serde(default)]
    pub structured_kind: Option<StructuredKind>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuredKind {
    Fields,
    Relationships,
    Chapters,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SearchTarget {
    Entry {
        entry_id: String,
    },
    Chapter {
        chapter_id: String,
        area: String,
    },
    Relationship {
        relationship_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        perspective_entry_id: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub key: String,
    pub title: String,
    pub context: String,
    pub workspace_state: String,
    pub reason: String,
    pub excerpt: String,
    pub preview: String,
    pub target: SearchTarget,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchGroup {
    pub kind: String,
    pub total: usize,
    pub hits: Vec<SearchHit>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub global_revision: i64,
    pub groups: Vec<SearchGroup>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntryAlias {
    pub id: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryAliases {
    pub global_revision: i64,
    pub aliases: Vec<EntryAlias>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AliasCommand {
    Add { text: String },
    Delete { alias_id: String },
}
