//! Ordered non-semantic pointers. No Relationship facts are created here.
use super::PersistenceError;
use crate::domain::{fields::*, structure::FieldId, EntryId};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}
struct Row {
    id: String,
    target: Option<String>,
    snapshot: Option<String>,
}
fn rows(conn: &Connection, entry: EntryId, field: FieldId) -> Result<Vec<Row>, PersistenceError> {
    let mut statement = conn.prepare(
        "SELECT id,target_record_id,unresolved_snapshot FROM entry_reference_value
         WHERE entry_id=?1 AND field_id=?2 ORDER BY ordinal,id",
    )?;
    let result = statement
        .query_map(params![entry.to_string(), field.to_string()], |r| {
            Ok(Row {
                id: r.get(0)?,
                target: r.get(1)?,
                snapshot: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result)
}
pub(super) fn read_value(
    conn: &Connection,
    entry: EntryId,
    field: FieldId,
) -> Result<Option<FieldValue>, PersistenceError> {
    let rows = rows(conn, entry, field)?;
    if rows.is_empty() {
        return Ok(None);
    }
    Ok(Some(FieldValue::EntryReferences(ReferenceSelection {
        expected_ids: rows.iter().map(|r| r.id.clone()).collect(),
        items: rows
            .into_iter()
            .map(|r| ReferenceItem::Existing { reference_id: r.id })
            .collect(),
    })))
}
pub(super) fn targets(
    conn: &Connection,
    entry: EntryId,
    field: FieldId,
) -> Result<Vec<ReferenceTarget>, PersistenceError> {
    let mut statement=conn.prepare("SELECT r.id,r.target_record_id,COALESCE(e.authored_name,json_extract(r.unresolved_snapshot,'$.label'),'[Unnamed Entry]'),i.workspace_state FROM entry_reference_value r LEFT JOIN entry e ON e.id=r.target_record_id LEFT JOIN record_identity i ON i.record_id=r.target_record_id WHERE r.entry_id=?1 AND r.field_id=?2 ORDER BY r.ordinal,r.id")?;
    let result = statement
        .query_map(params![entry.to_string(), field.to_string()], |r| {
            Ok(ReferenceTarget {
                id: r.get(0)?,
                target_id: r.get(1)?,
                label: r.get(2)?,
                workspace_state: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result)
}
pub(super) fn set(
    conn: &Connection,
    entry: EntryId,
    field: FieldId,
    value: Option<FieldValue>,
) -> Result<(), PersistenceError> {
    let prior = rows(conn, entry, field)?;
    let selection =
        match value {
            Some(FieldValue::EntryReferences(value)) => value,
            None if prior.is_empty() => return Ok(()),
            _ => return Err(invalid(
                "Reference changes need the reviewed selection. Reload the Field before retrying.",
            )),
        };
    if selection.expected_ids != prior.iter().map(|r| r.id.clone()).collect::<Vec<_>>() {
        return Err(invalid("These references changed. Your selection was not saved; reload and review it before retrying."));
    }
    let mut ids = HashSet::new();
    let mut targets = HashSet::new();
    let mut next = Vec::new();
    for item in selection.items {
        let row = match item {
            ReferenceItem::Existing { reference_id } => {
                let old = prior
                    .iter()
                    .find(|r| r.id == reference_id)
                    .ok_or_else(|| invalid("Reference does not belong to this Entry and Field"))?;
                Row {
                    id: old.id.clone(),
                    target: old.target.clone(),
                    snapshot: old.snapshot.clone(),
                }
            }
            ReferenceItem::Entry { entry_id } => {
                let target = entry_id.to_string();
                let state:Option<String>=conn.query_row("SELECT i.workspace_state FROM entry e JOIN record_identity i ON i.record_id=e.id WHERE e.id=?1 AND i.kind='entry'",[&target],|r|r.get(0)).optional()?;
                if state.as_deref() != Some("active") {
                    return Err(invalid("Choose an active Entry in this Project. Existing inactive references can be kept."));
                }
                let id = prior
                    .iter()
                    .find(|r| r.target.as_ref() == Some(&target))
                    .map(|r| r.id.clone())
                    .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
                Row {
                    id,
                    target: Some(target),
                    snapshot: None,
                }
            }
        };
        if !ids.insert(row.id.clone())
            || row
                .target
                .as_ref()
                .is_some_and(|id| !targets.insert(id.clone()))
        {
            return Err(invalid("An Entry can be selected only once in this Field"));
        }
        next.push(row);
    }
    // The caller's revision-guarded transaction covers validation and replacement.
    // Reinsert retained IDs to avoid transient ordinal collisions during reorder.
    conn.execute(
        "DELETE FROM entry_reference_value WHERE entry_id=?1 AND field_id=?2",
        params![entry.to_string(), field.to_string()],
    )?;
    for (ordinal, row) in next.into_iter().enumerate() {
        conn.execute("INSERT INTO entry_reference_value(id,entry_id,field_id,target_record_id,target_kind,unresolved_snapshot,ordinal) VALUES(?1,?2,?3,?4,'entry',?5,?6)",
            params![row.id,entry.to_string(),field.to_string(),row.target,row.snapshot,ordinal as i64])?;
    }
    Ok(())
}
