//! Chapters and versioned manuscript documents remain distinct from world Entries.
use super::{structure::ChapterId, EntryId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;

pub const DOCUMENT_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DocumentArea {
    Manuscript,
    Plan,
    Notes,
}
impl DocumentArea {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manuscript => "manuscript",
            Self::Plan => "plan",
            Self::Notes => "notes",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentEdit {
    pub area: DocumentArea,
    pub schema_version: i64,
    pub content: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RichDocument {
    pub id: String,
    pub area: DocumentArea,
    pub schema_version: i64,
    pub content: Option<Value>,
    pub plain_text: String,
    pub word_count: usize,
    pub revision: i64,
    pub read_only_reason: Option<String>,
    pub original_json: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSummary {
    pub id: ChapterId,
    pub title: String,
    pub workspace_state: String,
    pub reading_rank: i64,
    pub revision: i64,
    pub word_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryRole {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryLink {
    pub id: String,
    pub entry_id: Option<EntryId>,
    pub label: String,
    pub workspace_state: String,
    pub roles: Vec<StoryRole>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSnapshot {
    pub global_revision: i64,
    pub chapter: ChapterSummary,
    pub documents: Vec<RichDocument>,
    pub links: Vec<StoryLink>,
    pub roles: Vec<StoryRole>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryIndex {
    pub global_revision: i64,
    pub chapters: Vec<ChapterSummary>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryUsage {
    pub chapter: ChapterSummary,
    pub roles: Vec<StoryRole>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StoryCommand {
    Create {
        title: String,
    },
    Save {
        chapter_id: ChapterId,
        title: Option<String>,
        documents: Vec<DocumentEdit>,
    },
    Move {
        chapter_id: ChapterId,
        before_id: Option<ChapterId>,
    },
    SetState {
        chapter_id: ChapterId,
        state: WorkspaceState,
    },
    SetLink {
        chapter_id: ChapterId,
        entry_id: EntryId,
        role_ids: Vec<String>,
    },
    Unlink {
        chapter_id: ChapterId,
        link_id: String,
    },
    CreateRole {
        chapter_id: ChapterId,
        name: String,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceState {
    Active,
    Archived,
    Trashed,
}
impl WorkspaceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Trashed => "trashed",
        }
    }
}

/// A deliberately bounded subset shared with the configured Tiptap StarterKit.
/// Reject unknown nodes/attributes instead of silently dropping authored content.
pub fn document_text(version: i64, value: &Value) -> Result<(String, usize), String> {
    if version != DOCUMENT_VERSION {
        return Err(
            "This document version is not supported. Its original content is preserved.".into(),
        );
    }
    if value.to_string().len() > 16 * 1024 * 1024 {
        return Err("This document is too large to edit safely.".into());
    }
    let mut text = String::new();
    let mut nodes = 0;
    validate_node(value, "root", 0, &mut nodes, &mut text)?;
    let count = text.unicode_words().count();
    Ok((text, count))
}
fn validate_node(
    value: &Value,
    parent: &str,
    depth: usize,
    nodes: &mut usize,
    text: &mut String,
) -> Result<(), String> {
    let invalid = || {
        "Unsupported or damaged rich-text content. The original document is preserved.".to_string()
    };
    *nodes += 1;
    if depth > 64 || *nodes > 100_000 {
        return Err(invalid());
    }
    let node = value.as_object().ok_or_else(invalid)?;
    let kind = node
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    let inline = matches!(kind, "text" | "hardBreak");
    let block = matches!(
        kind,
        "paragraph" | "heading" | "blockquote" | "bulletList" | "orderedList" | "horizontalRule"
    );
    let fits = match parent {
        "root" => kind == "doc",
        "doc" | "blockquote" | "listItem" => block,
        "paragraph" | "heading" => inline,
        "bulletList" | "orderedList" => kind == "listItem",
        _ => false,
    };
    if !fits
        || node.keys().any(|key| {
            !matches!(
                key.as_str(),
                "type" | "content" | "text" | "marks" | "attrs"
            )
        })
    {
        return Err(invalid());
    }
    if let Some(attrs) = node.get("attrs") {
        let attrs = attrs.as_object().ok_or_else(invalid)?;
        for (key, val) in attrs {
            let valid = match (kind, key.as_str()) {
                ("heading", "level") => matches!(val.as_i64(), Some(1..=3)),
                ("orderedList", "start") => {
                    val.as_i64().is_some_and(|n| n > 0 && n <= i32::MAX as i64)
                }
                ("orderedList", "type") => {
                    val.is_null()
                        || val
                            .as_str()
                            .is_some_and(|v| ["1", "a", "A", "i", "I"].contains(&v))
                }
                _ => false,
            };
            if !valid {
                return Err(invalid());
            }
        }
    }
    if kind == "heading" && value.pointer("/attrs/level").is_none() {
        return Err(invalid());
    }
    if let Some(marks) = node.get("marks") {
        if !inline {
            return Err(invalid());
        }
        for mark in marks.as_array().ok_or_else(invalid)? {
            let m = mark.as_object().ok_or_else(invalid)?;
            if m.len() != 1
                || !matches!(
                    m.get("type").and_then(Value::as_str),
                    Some("bold" | "italic" | "strike" | "underline" | "code")
                )
            {
                return Err(invalid());
            }
        }
    }
    if kind == "text" {
        let s = node
            .get("text")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(invalid)?;
        if node.contains_key("content") {
            return Err(invalid());
        }
        text.push_str(s);
        return Ok(());
    }
    if node.contains_key("text") {
        return Err(invalid());
    }
    let children = match node.get("content") {
        Some(c) => c.as_array().ok_or_else(invalid)?.as_slice(),
        None => &[],
    };
    if matches!(kind, "hardBreak" | "horizontalRule") && !children.is_empty() {
        return Err(invalid());
    }
    if matches!(
        kind,
        "doc" | "blockquote" | "bulletList" | "orderedList" | "listItem"
    ) && children.is_empty()
    {
        return Err(invalid());
    }
    if kind == "listItem" && children[0].get("type").and_then(Value::as_str) != Some("paragraph") {
        return Err(invalid());
    }
    for child in children {
        validate_node(child, kind, depth + 1, nodes, text)?;
    }
    if matches!(
        kind,
        "paragraph" | "heading" | "hardBreak" | "horizontalRule"
    ) {
        text.push('\n');
    }
    Ok(())
}
