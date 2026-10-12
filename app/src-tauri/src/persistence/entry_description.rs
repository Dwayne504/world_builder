use super::PersistenceError;
use crate::domain::{entry_description::*, story::document_text, EntryId};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::Value;

fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}

pub(super) fn read(
    conn: &Connection,
    entry_id: EntryId,
) -> Result<EntryDescriptionSnapshot, PersistenceError> {
    let workspace_state = conn.query_row(
        "SELECT i.workspace_state FROM entry e JOIN record_identity i ON i.record_id=e.id WHERE e.id=?1",
        [entry_id.to_string()], |r| r.get(0),
    ).optional()?.ok_or_else(|| invalid("Entry not found in this Project"))?;
    let stored = conn.query_row(
        "SELECT id,document_schema_version,canonical_json,plain_text,word_count,revision,migration_state FROM rich_document WHERE owner_kind='entry' AND owner_id=?1 AND area='description'",
        [entry_id.to_string()], |r| Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,usize>(4)?,r.get::<_,i64>(5)?,r.get::<_,String>(6)?)),
    ).optional()?;
    let document = stored.map(
        |(id, schema_version, json, plain_text, word_count, revision, migration_state)| {
            let parsed = serde_json::from_str::<Value>(&json);
            let validated = parsed
                .as_ref()
                .map_err(|_| {
                    "This description is damaged. Its original content is preserved.".to_string()
                })
                .and_then(|v| document_text(schema_version, v));
            let read_only_reason = if migration_state != "current" {
                Some("This description needs recovery. Its original content is preserved.".into())
            } else {
                validated.as_ref().err().cloned()
            };
            let (plain_text, word_count) = validated.unwrap_or((plain_text, word_count));
            EntryDescriptionDocument {
                id,
                schema_version,
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
            }
        },
    );
    Ok(EntryDescriptionSnapshot {
        global_revision: conn.query_row(
            "SELECT last_committed_revision FROM project_meta WHERE id=1",
            [],
            |r| r.get(0),
        )?,
        entry_id,
        workspace_state,
        document,
    })
}

pub(super) fn save(
    conn: &mut Connection,
    entry_id: EntryId,
    expected: i64,
    expected_document_revision: Option<i64>,
    schema_version: i64,
    content: Value,
) -> Result<EntryDescriptionSnapshot, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let before = read(&tx, entry_id)?;
    if expected != before.global_revision {
        return Err(PersistenceError::StaleRevision {
            expected,
            current: before.global_revision,
        });
    }
    if before.document.as_ref().map(|d| d.revision) != expected_document_revision {
        return Err(PersistenceError::StaleDocumentRevision);
    }
    if before.workspace_state != "active" {
        return Err(invalid("Restore this Entry before editing its description"));
    }
    if let Some(reason) = before
        .document
        .as_ref()
        .and_then(|d| d.read_only_reason.as_ref())
    {
        return Err(invalid(reason));
    }
    let (plain, count) = document_text(schema_version, &content).map_err(invalid)?;
    let now = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO rich_document (id,owner_kind,owner_id,area,document_schema_version,canonical_json,plain_text,word_count,migration_state,created_at,updated_at,revision)
         VALUES(?1,'entry',?2,'description',?3,?4,?5,?6,'current',?7,?7,1)
         ON CONFLICT(owner_kind,owner_id,area) DO UPDATE SET canonical_json=excluded.canonical_json,plain_text=excluded.plain_text,word_count=excluded.word_count,updated_at=excluded.updated_at,revision=rich_document.revision+1",
        params![uuid::Uuid::now_v7().to_string(),entry_id.to_string(),schema_version,content.to_string(),plain,count,now],
    )?;
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1",[now])?;
    let result = read(&tx, entry_id)?;
    tx.commit()?;
    Ok(result)
}
