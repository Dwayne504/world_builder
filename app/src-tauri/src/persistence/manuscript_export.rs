use super::PersistenceError;
use crate::domain::{manuscript_export::*, structure::ChapterId};
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashSet;

pub(super) fn read(
    conn: &Connection,
    chapter_ids: Vec<ChapterId>,
) -> Result<ManuscriptSelection, PersistenceError> {
    let invalid = |message: &str| PersistenceError::Other(message.into());
    let selected: HashSet<_> = chapter_ids.iter().copied().collect();
    if selected.is_empty() || selected.len() != chapter_ids.len() || selected.len() > 10_000 {
        return Err(invalid("Choose one or more different Chapters to export."));
    }
    // One worker job and one read transaction give the preview a coherent
    // revision, reading order and manuscript even if an external reader exists.
    let transaction = conn.unchecked_transaction()?;
    let index = super::story::index(&transaction)?;
    let working_name = transaction.query_row(
        "SELECT working_name FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let mut ordered = index
        .chapters
        .into_iter()
        .filter(|c| selected.contains(&c.id))
        .collect::<Vec<_>>();
    if ordered.len() != selected.len() {
        return Err(invalid(
            "A selected Chapter is not in this Project. Refresh the export selection.",
        ));
    }
    ordered.sort_by(|a, b| {
        a.reading_rank
            .cmp(&b.reading_rank)
            .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
    });
    let mut result = ManuscriptSelection {
        global_revision: index.global_revision,
        working_name,
        chapters: vec![],
        markdown: String::new(),
        word_count: 0,
        format_notes: vec![],
    };
    let mut uses_html = false;
    for chapter in ordered {
        let document: Option<(i64, String, String)> = transaction.query_row(
            "SELECT document_schema_version,canonical_json,migration_state FROM rich_document WHERE owner_kind='story_unit' AND owner_id=?1 AND area='manuscript'",
            [chapter.id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))
        ).optional()?;
        let title = if chapter.title.trim().is_empty() {
            "[Unnamed Chapter]".into()
        } else {
            chapter.title
        };
        let (markdown, word_count, html) = match document {
            None => (String::new(), 0, false),
            Some((version, json, state)) => {
                if state != "current" {
                    return Err(invalid("A selected manuscript needs recovery and cannot be exported safely. Its original content is unchanged."));
                }
                let content = serde_json::from_str(&json).map_err(|_| invalid("A selected manuscript is damaged and cannot be exported safely. Its original content is unchanged."))?;
                render_manuscript(version, &content)
                    .map_err(|reason| invalid(&format!("Cannot export {title}: {reason}")))?
            }
        };
        result
            .markdown
            .push_str(&format!("# {}\n\n{}", escape_markdown(&title), markdown));
        // Bound this transient preview instead of allocating an unlimited IPC payload.
        if result.markdown.len() > 64 * 1024 * 1024 {
            return Err(invalid(
                "This export is too large for one preview. Select fewer Chapters.",
            ));
        }
        result.word_count += word_count;
        result.chapters.push(ExportChapter {
            id: chapter.id,
            title,
            workspace_state: chapter.workspace_state,
            word_count,
        });
        uses_html |= html;
    }
    if uses_html {
        result.format_notes.push("Formatting is preserved with safe HTML inside Markdown. Some Markdown readers do not show underline or styled numbering.".into());
    }
    transaction.commit()?;
    Ok(result)
}
