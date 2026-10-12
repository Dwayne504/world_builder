//! Definition retirement changes future choices, never existing assignments.
use super::PersistenceError;
use crate::domain::{lifecycle::*, structure::FieldId, CategoryId, EntryId, TypeId};
use rusqlite::{Connection, OptionalExtension};
fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}

pub(super) fn validate_assignment(
    conn: &Connection,
    category: &str,
    selected: Option<TypeId>,
    existing: Option<TypeId>,
) -> Result<(), PersistenceError> {
    let Some(id) = selected else {
        return Ok(());
    };
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT category_id,retired_at FROM type_def WHERE id=?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((owner, retired)) = row else {
        return Err(invalid("Type not found in this Project"));
    };
    if owner != category {
        return Err(invalid(
            "The selected Type does not belong to the selected Category.",
        ));
    }
    if retired.is_some() && selected != existing {
        return Err(invalid(
            "This Type is retired. Restore it before choosing it for a new Entry or child Type.",
        ));
    }
    Ok(())
}

pub(super) fn preview(
    conn: &Connection,
    id: TypeId,
) -> Result<TypeRetirementPreview, PersistenceError> {
    let tx = conn.unchecked_transaction()?;
    let result = read(&tx, id)?;
    tx.commit()?;
    Ok(result)
}
const DESCENDANTS:&str="WITH RECURSIVE descendants(id) AS (SELECT id FROM type_def WHERE id=?1 UNION SELECT t.id FROM type_def t JOIN descendants d ON t.parent_type_id=d.id)";
fn read(conn: &Connection, id: TypeId) -> Result<TypeRetirementPreview, PersistenceError> {
    let (name, category, retired_at): (String, String, Option<String>) = conn
        .query_row(
            "SELECT name,category_id,retired_at FROM type_def WHERE id=?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| invalid("Type not found in this Project"))?;
    let global_revision = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let mut statement=conn.prepare(&format!("{DESCENDANTS} SELECT t.id,t.name,t.parent_type_id,t.retired_at FROM type_def t JOIN descendants d ON d.id=t.id WHERE t.id<>?1 ORDER BY t.name COLLATE NOCASE,t.id"))?;
    let descendant_types = statement
        .query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?
        .map(|row| {
            let (id, name, parent, retired_at) = row?;
            Ok(TypeDescendant {
                id: TypeId::parse(&id).map_err(invalid)?,
                name,
                parent_type_id: parent
                    .map(|p| TypeId::parse(&p).map_err(invalid))
                    .transpose()?,
                retired_at,
            })
        })
        .collect::<Result<Vec<_>, PersistenceError>>()?;
    let mut statement=conn.prepare(&format!("{DESCENDANTS} SELECT e.id,COALESCE(e.authored_name,'[Unnamed Entry]'),i.workspace_state,t.id,t.name FROM entry e JOIN record_identity i ON i.record_id=e.id JOIN type_def t ON t.id=e.type_id JOIN descendants d ON d.id=t.id ORDER BY e.authored_name COLLATE NOCASE,e.id"))?;
    let assigned_entries = statement
        .query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .map(|row| {
            let (id, label, workspace_state, kind, type_name) = row?;
            Ok(TypeAssignedEntry {
                id: EntryId::parse(&id).map_err(invalid)?,
                label,
                workspace_state,
                type_id: TypeId::parse(&kind).map_err(invalid)?,
                type_name,
            })
        })
        .collect::<Result<Vec<_>, PersistenceError>>()?;
    let direct_entry_count = assigned_entries
        .iter()
        .filter(|entry| entry.type_id == id)
        .count();
    let inherited_entry_count = assigned_entries.len() - direct_entry_count;
    let mut statement=conn.prepare("SELECT f.id,f.name,f.value_kind FROM field_definition f JOIN field_availability a ON a.field_id=f.id WHERE a.provider_kind='type' AND a.provider_id=?1 ORDER BY f.name COLLATE NOCASE,f.id")?;
    let default_fields = statement
        .query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .map(|row| {
            let (id, name, kind) = row?;
            Ok(TypeDefaultField {
                id: FieldId::parse(&id).map_err(invalid)?,
                name,
                kind,
            })
        })
        .collect::<Result<Vec<_>, PersistenceError>>()?;
    Ok(TypeRetirementPreview {
        global_revision,
        type_id: id,
        category_id: CategoryId::parse(&category).map_err(invalid)?,
        name,
        retired_at,
        assigned_entries,
        descendant_types,
        direct_entry_count,
        inherited_entry_count,
        default_fields,
    })
}

pub(super) fn set_retired(
    conn: &Connection,
    id: TypeId,
    retired: bool,
    now: &str,
) -> Result<(), PersistenceError> {
    // Called inside the revision-guarded structure transaction. The preview
    // revision must still match before any lifecycle operation reaches here.
    let current: Option<Option<String>> = conn
        .query_row(
            "SELECT retired_at FROM type_def WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    let current = current.ok_or_else(|| invalid("Type not found in this Project"))?;
    if current.is_some() == retired {
        return Err(invalid(if retired {
            "This Type is already retired. Refresh its settings."
        } else {
            "This Type is already available. Refresh its settings."
        }));
    }
    conn.execute(
        "UPDATE type_def SET retired_at=?2,revision=revision+1,updated_at=?3 WHERE id=?1",
        rusqlite::params![id.to_string(), retired.then_some(now), now],
    )?;
    Ok(())
}
