use super::PersistenceError;
use crate::domain::{require_definition_name, story::*, structure::ChapterId, EntryId};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::Value;
use uuid::Uuid;

fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}
fn revision(conn: &Connection) -> Result<i64, PersistenceError> {
    Ok(conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?)
}
const SUMMARY: &str = "SELECT s.id,s.title,i.workspace_state,s.reading_rank,s.revision,COALESCE(d.word_count,0) FROM story_unit s JOIN record_identity i ON i.record_id=s.id LEFT JOIN rich_document d ON d.owner_id=s.id AND d.area='manuscript'";
fn summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChapterSummary> {
    let id: String = row.get(0)?;
    Ok(ChapterSummary {
        id: ChapterId::parse(&id).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        title: row.get(1)?,
        workspace_state: row.get(2)?,
        reading_rank: row.get(3)?,
        revision: row.get(4)?,
        word_count: row.get(5)?,
    })
}
pub(super) fn index(conn: &Connection) -> Result<StoryIndex, PersistenceError> {
    let chapters = conn
        .prepare(&format!("{SUMMARY} ORDER BY s.reading_rank"))?
        .query_map([], summary)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(StoryIndex {
        global_revision: revision(conn)?,
        chapters,
    })
}
fn roles(conn: &Connection, link: Option<&str>) -> Result<Vec<StoryRole>, PersistenceError> {
    let sql = if link.is_some() {
        "SELECT r.id,r.name FROM story_role r JOIN story_link_role lr ON lr.role_id=r.id WHERE lr.link_id=?1 ORDER BY r.name,r.id"
    } else {
        "SELECT id,name FROM story_role WHERE retired_at IS NULL ORDER BY name,id"
    };
    let mut statement = conn.prepare(sql)?;
    let params = link.into_iter().collect::<Vec<_>>();
    let result = statement
        .query_map(rusqlite::params_from_iter(params), |r| {
            Ok(StoryRole {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(result)
}
pub(super) fn read(conn: &Connection, id: ChapterId) -> Result<ChapterSnapshot, PersistenceError> {
    let chapter = conn
        .query_row(
            &format!("{SUMMARY} WHERE s.id=?1"),
            [id.to_string()],
            summary,
        )
        .optional()?
        .ok_or_else(|| invalid("Chapter not found in this Project"))?;
    let mut statement = conn.prepare("SELECT id,area,document_schema_version,canonical_json,plain_text,word_count,revision,migration_state FROM rich_document WHERE owner_id=?1 ORDER BY area")?;
    let documents = statement
        .query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, usize>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?
        .map(|row| {
            let (id, area, version, json, plain_text, word_count, revision, migration_state) = row?;
            let area = match area.as_str() {
                "manuscript" => DocumentArea::Manuscript,
                "plan" => DocumentArea::Plan,
                "notes" => DocumentArea::Notes,
                _ => return Err(invalid("Unknown document area")),
            };
            let parsed = serde_json::from_str::<Value>(&json);
            let validated = parsed
                .as_ref()
                .map_err(|e| e.to_string())
                .and_then(|v| document_text(version, v));
            let read_only_reason = if migration_state != "current" {
                Some("This document needs recovery. Its original content is preserved.".into())
            } else {
                validated.as_ref().err().cloned()
            };
            // Reading never migrates or overwrites a damaged/newer document. Other areas stay usable.
            let (plain_text, word_count) = validated.unwrap_or((plain_text, word_count));
            Ok(RichDocument {
                id,
                area,
                schema_version: version,
                content: if read_only_reason.is_none() {
                    parsed.ok()
                } else {
                    None
                },
                plain_text,
                word_count,
                revision,
                original_json: read_only_reason.as_ref().map(|_| json),
                read_only_reason,
            })
        })
        .collect::<Result<Vec<_>, PersistenceError>>()?;
    let mut statement = conn.prepare("SELECT l.id,l.entry_id,COALESCE(e.authored_name,'[Unnamed Entry]'),COALESCE(i.workspace_state,'unresolved'),l.unresolved_snapshot FROM story_link l LEFT JOIN entry e ON e.id=l.entry_id LEFT JOIN record_identity i ON i.record_id=l.entry_id WHERE l.story_unit_id=?1 ORDER BY COALESCE(e.authored_name,''),l.id")?;
    let links = statement
        .query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?
        .map(|row| {
            let (id, entry, label, workspace_state, unresolved) = row?;
            let label = if entry.is_none() {
                unresolved
                    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                    .and_then(|v| v.get("label").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| "Deleted Entry".into())
            } else {
                label
            };
            Ok(StoryLink {
                roles: roles(conn, Some(&id))?,
                id,
                entry_id: entry
                    .map(|s| EntryId::parse(&s).map_err(invalid))
                    .transpose()?,
                label,
                workspace_state,
            })
        })
        .collect::<Result<Vec<_>, PersistenceError>>()?;
    Ok(ChapterSnapshot {
        global_revision: revision(conn)?,
        chapter,
        documents,
        links,
        roles: roles(conn, None)?,
    })
}
pub(super) fn usage(
    conn: &Connection,
    entry: EntryId,
) -> Result<Vec<StoryUsage>, PersistenceError> {
    let mut statement=conn.prepare(&format!("{SUMMARY} JOIN story_link l ON l.story_unit_id=s.id WHERE l.entry_id=?1 ORDER BY s.reading_rank"))?;
    let chapters = statement
        .query_map([entry.to_string()], summary)?
        .collect::<Result<Vec<_>, _>>()?;
    chapters
        .into_iter()
        .map(|chapter| {
            let link: String = conn.query_row(
                "SELECT id FROM story_link WHERE story_unit_id=?1 AND entry_id=?2",
                params![chapter.id.to_string(), entry.to_string()],
                |r| r.get(0),
            )?;
            Ok(StoryUsage {
                chapter,
                roles: roles(conn, Some(&link))?,
            })
        })
        .collect()
}
fn active(conn: &Connection, id: ChapterId) -> Result<(), PersistenceError> {
    let active:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM story_unit s JOIN record_identity i ON i.record_id=s.id WHERE s.id=?1 AND i.workspace_state='active')",[id.to_string()],|r|r.get(0))?;
    if active {
        Ok(())
    } else {
        Err(invalid("Restore this Chapter before editing it"))
    }
}
fn title(value: String) -> Result<String, PersistenceError> {
    if value.chars().count() > 1000 {
        return Err(invalid("Chapter titles may contain up to 1,000 characters"));
    }
    Ok(value)
}
pub(super) fn apply(
    conn: &mut Connection,
    expected: i64,
    command: StoryCommand,
) -> Result<ChapterSnapshot, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = revision(&tx)?;
    if expected != current {
        return Err(PersistenceError::StaleRevision { expected, current });
    }
    let now = chrono::Utc::now().to_rfc3339();
    let id = match command {
        StoryCommand::Create { title: value } => {
            let id = ChapterId::new();
            tx.execute(
                "INSERT INTO record_identity VALUES(?1,'story_unit','active',?2,?2)",
                params![id.to_string(), now],
            )?;
            tx.execute("INSERT INTO story_unit VALUES(?1,'chapter',?2,(SELECT COALESCE(MAX(reading_rank),0)+1 FROM story_unit),?3,?3,1)",params![id.to_string(),title(value)?,now])?;
            for area in [
                DocumentArea::Manuscript,
                DocumentArea::Plan,
                DocumentArea::Notes,
            ] {
                tx.execute("INSERT INTO rich_document VALUES(?1,'story_unit',?2,?3,1,?4,'',0,'current',?5,?5,1)",params![Uuid::now_v7().to_string(),id.to_string(),area.as_str(),r#"{"type":"doc","content":[{"type":"paragraph"}]}"#,now])?;
            }
            id
        }
        StoryCommand::Save {
            chapter_id,
            title: chapter_title,
            documents,
        } => {
            active(&tx, chapter_id)?;
            let mut seen = std::collections::HashSet::new();
            for edit in documents {
                if !seen.insert(edit.area) {
                    return Err(invalid("Document area supplied more than once"));
                }
                let existing = read(&tx, chapter_id)?
                    .documents
                    .into_iter()
                    .find(|d| d.area == edit.area)
                    .ok_or_else(|| invalid("Document is missing; existing content is preserved"))?;
                if let Some(reason) = existing.read_only_reason {
                    return Err(invalid(reason));
                }
                let (plain, count) =
                    document_text(edit.schema_version, &edit.content).map_err(invalid)?;
                tx.execute("UPDATE rich_document SET canonical_json=?1,plain_text=?2,word_count=?3,revision=revision+1,updated_at=?4 WHERE id=?5",params![edit.content.to_string(),plain,count,now,existing.id])?;
            }
            if let Some(value) = chapter_title {
                tx.execute(
                    "UPDATE story_unit SET title=?1 WHERE id=?2",
                    params![title(value)?, chapter_id.to_string()],
                )?;
            }
            chapter_id
        }
        StoryCommand::Move {
            chapter_id,
            before_id,
        } => {
            active(&tx, chapter_id)?;
            if let Some(before) = before_id {
                active(&tx, before)?;
                if before == chapter_id {
                    return Err(invalid("A Chapter cannot be moved before itself"));
                }
            }
            let mut ordered = index(&tx)?
                .chapters
                .into_iter()
                .map(|c| c.id)
                .filter(|id| *id != chapter_id)
                .collect::<Vec<_>>();
            let position = before_id
                .and_then(|id| ordered.iter().position(|v| *v == id))
                .unwrap_or(ordered.len());
            ordered.insert(position, chapter_id);
            // Temporary negative ranks avoid UNIQUE collisions; the transaction hides this phase.
            tx.execute("UPDATE story_unit SET reading_rank=-reading_rank", [])?;
            for (rank, id) in ordered.iter().enumerate() {
                tx.execute(
                    "UPDATE story_unit SET reading_rank=?1 WHERE id=?2",
                    params![rank + 1, id.to_string()],
                )?;
            }
            chapter_id
        }
        StoryCommand::SetState { chapter_id, state } => {
            read(&tx, chapter_id)?;
            tx.execute("UPDATE record_identity SET workspace_state=?1,lifecycle_changed_at=?2 WHERE record_id=?3",params![state.as_str(),now,chapter_id.to_string()])?;
            chapter_id
        }
        StoryCommand::SetLink {
            chapter_id,
            entry_id,
            role_ids,
        } => {
            active(&tx, chapter_id)?;
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM entry e JOIN record_identity i ON i.record_id=e.id WHERE e.id=?1 AND i.workspace_state='active')",[entry_id.to_string()],|r|r.get(0))?;
            if !exists {
                return Err(invalid("Choose an active Entry in this Project"));
            }
            tx.execute("INSERT INTO story_link VALUES(?1,?2,?3,NULL,?4,?4,1) ON CONFLICT(story_unit_id,entry_id) DO UPDATE SET updated_at=excluded.updated_at,revision=story_link.revision+1",params![Uuid::now_v7().to_string(),chapter_id.to_string(),entry_id.to_string(),now])?;
            let link: String = tx.query_row(
                "SELECT id FROM story_link WHERE story_unit_id=?1 AND entry_id=?2",
                params![chapter_id.to_string(), entry_id.to_string()],
                |r| r.get(0),
            )?;
            tx.execute("DELETE FROM story_link_role WHERE link_id=?1", [&link])?;
            for role in role_ids {
                let valid: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM story_role WHERE id=?1 AND retired_at IS NULL)",
                    [&role],
                    |r| r.get(0),
                )?;
                if !valid {
                    return Err(invalid("Story Role not found in this Project"));
                }
                tx.execute(
                    "INSERT INTO story_link_role VALUES(?1,?2) ON CONFLICT DO NOTHING",
                    params![link, role],
                )?;
            }
            chapter_id
        }
        StoryCommand::Unlink {
            chapter_id,
            link_id,
        } => {
            active(&tx, chapter_id)?;
            if tx.execute(
                "DELETE FROM story_link WHERE id=?1 AND story_unit_id=?2",
                params![link_id, chapter_id.to_string()],
            )? != 1
            {
                return Err(invalid("Story link not found in this Chapter"));
            }
            chapter_id
        }
        StoryCommand::CreateRole { chapter_id, name } => {
            active(&tx, chapter_id)?;
            let name = require_definition_name(&name).map_err(invalid)?;
            tx.execute(
                "INSERT INTO story_role VALUES(?1,?2,?3,?3,1,NULL)",
                params![Uuid::now_v7().to_string(), name, now],
            )?;
            chapter_id
        }
    };
    tx.execute(
        "UPDATE story_unit SET updated_at=?1,revision=revision+1 WHERE id=?2",
        params![now, id.to_string()],
    )?;
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1",[now])?;
    let result = read(&tx, id)?;
    tx.commit()?;
    Ok(result)
}
