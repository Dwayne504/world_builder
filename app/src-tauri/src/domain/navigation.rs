use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationKind {
    Entry,
    StoryUnit,
    TemporalOccurrence,
}
impl NavigationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::StoryUnit => "story_unit",
            Self::TemporalOccurrence => "temporal_occurrence",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationTarget {
    pub record_kind: NavigationKind,
    pub record_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationRecord {
    #[serde(flatten)]
    pub target: NavigationTarget,
    pub label: String,
    pub workspace_state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationSnapshot {
    pub pins: Vec<NavigationRecord>,
    pub recents: Vec<NavigationRecord>,
    pub recent_limit: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum NavigationCommand {
    Visit {
        target: NavigationTarget,
    },
    Pin {
        target: NavigationTarget,
        pinned: bool,
    },
    SetRecentLimit {
        limit: u32,
    },
    ClearRecents,
}
