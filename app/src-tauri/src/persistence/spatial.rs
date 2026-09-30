use super::PersistenceError;
use crate::domain::{authored_name, spatial::*, EntryId};
use rusqlite::{params, Connection, TransactionBehavior};

fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}

pub(super) fn read(conn: &Connection) -> Result<SpatialSnapshot, PersistenceError> {
    let global_revision = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let mut statement = conn.prepare("SELECT e.id, COALESCE(e.authored_name,'[Unnamed Entry]'), i.workspace_state, s.entry_id IS NOT NULL, s.primary_parent_id FROM entry e JOIN record_identity i ON i.record_id=e.id LEFT JOIN spatial_node s ON s.entry_id=e.id ORDER BY COALESCE(e.authored_name,'[Unnamed Entry]') COLLATE NOCASE, e.id")?;
    let entries = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?
        .map(|row| {
            let (id, label, workspace_state, spatial, parent) = row?;
            Ok(SpatialEntry {
                id: EntryId::parse(&id).map_err(invalid)?,
                label,
                workspace_state,
                spatial,
                parent_id: parent
                    .map(|id| EntryId::parse(&id).map_err(invalid))
                    .transpose()?,
            })
        })
        .collect::<Result<_, PersistenceError>>()?;
    let mut statement = conn.prepare("SELECT 'category',category_id FROM category_capability_default WHERE capability_id='spatial' UNION ALL SELECT 'type',type_id FROM type_capability_default WHERE capability_id='spatial'")?;
    let defaults = statement
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .map(|row| {
            let (kind, id) = row?;
            Ok(if kind == "category" {
                CapabilityProvider::Category {
                    id: crate::domain::CategoryId::parse(&id).map_err(invalid)?,
                }
            } else {
                CapabilityProvider::Type {
                    id: crate::domain::TypeId::parse(&id).map_err(invalid)?,
                }
            })
        })
        .collect::<Result<_, PersistenceError>>()?;
    Ok(SpatialSnapshot {
        global_revision,
        entries,
        defaults,
    })
}

fn active(conn: &Connection, id: EntryId, spatial: bool) -> Result<(), PersistenceError> {
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM entry e JOIN record_identity i ON i.record_id=e.id WHERE e.id=?1 AND i.workspace_state='active' AND (?2=0 OR EXISTS(SELECT 1 FROM spatial_node WHERE entry_id=e.id)))", params![id.to_string(), spatial], |r| r.get(0))?;
    if !exists {
        return Err(invalid(if spatial {
            "Choose an active Spatial Entry in this Project"
        } else {
            "Choose an active Entry in this Project"
        }));
    }
    Ok(())
}

fn enable(conn: &Connection, id: EntryId, now: &str) -> Result<(), PersistenceError> {
    active(conn, id, false)?;
    conn.execute(
        "INSERT INTO entry_capability VALUES (?1,'spatial',?2,?2,1) ON CONFLICT DO NOTHING",
        params![id.to_string(), now],
    )?;
    Ok(())
}

fn reparent(
    conn: &Connection,
    id: EntryId,
    parent: Option<EntryId>,
    now: &str,
) -> Result<(), PersistenceError> {
    active(conn, id, true)?;
    if let Some(parent) = parent {
        active(conn, parent, true)?;
    }
    // The recursive database trigger guards this transaction as well as direct SQL.
    conn.execute("UPDATE spatial_node SET primary_parent_id=?1,updated_at=?2,revision=revision+1 WHERE entry_id=?3", params![parent.map(|id|id.to_string()),now,id.to_string()])?;
    Ok(())
}

pub(super) fn apply(
    conn: &mut Connection,
    expected: i64,
    command: SpatialCommand,
) -> Result<SpatialSnapshot, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current: i64 = tx.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    if current != expected {
        return Err(PersistenceError::StaleRevision { expected, current });
    }
    let now = chrono::Utc::now().to_rfc3339();
    match command {
        SpatialCommand::SetEnabled { entry_id, enabled } => {
            active(&tx, entry_id, false)?;
            if enabled {
                enable(&tx, entry_id, &now)?;
            } else {
                let dependent: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM spatial_node WHERE primary_parent_id=?1 OR (entry_id=?1 AND primary_parent_id IS NOT NULL))", [entry_id.to_string()], |r|r.get(0))?;
                if dependent {
                    return Err(invalid("Move this Entry to the top level and move its children elsewhere before removing Spatial."));
                }
                tx.execute(
                    "DELETE FROM spatial_node WHERE entry_id=?1",
                    [entry_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM entry_capability WHERE entry_id=?1 AND capability_id='spatial'",
                    [entry_id.to_string()],
                )?;
            }
        }
        SpatialCommand::Reparent {
            entry_id,
            parent_id,
        } => reparent(&tx, entry_id, parent_id, &now)?,
        SpatialCommand::CreateChild {
            parent_id,
            name,
            category_id,
            type_id,
        } => {
            active(&tx, parent_id, true)?;
            let id = EntryId::new();
            let category = match category_id {
                Some(id) => id.to_string(),
                None => tx.query_row(
                    "SELECT id FROM category WHERE is_uncategorized=1",
                    [],
                    |r| r.get(0),
                )?,
            };
            let name = authored_name(name);
            tx.execute("INSERT INTO record_identity(record_id,kind,workspace_state,lifecycle_changed_at,created_at) VALUES(?1,'entry','active',?2,?2)", params![id.to_string(),now])?;
            tx.execute("INSERT INTO entry(id,category_id,type_id,authored_name,created_at,updated_at,revision) VALUES(?1,?2,?3,?4,?5,?5,1)", params![id.to_string(),category,type_id.map(|id|id.to_string()),name,now])?;
            enable(&tx, id, &now)?;
            reparent(&tx, id, Some(parent_id), &now)?;
        }
        SpatialCommand::SetDefault { provider, enabled } => {
            let (table, column, id) = match provider {
                CapabilityProvider::Category { id } => {
                    ("category_capability_default", "category_id", id.to_string())
                }
                CapabilityProvider::Type { id } => {
                    ("type_capability_default", "type_id", id.to_string())
                }
            };
            // Only fixed table/column names enter the SQL; authored IDs are parameters.
            if enabled {
                tx.execute(
                    &format!(
                        "INSERT INTO {table} VALUES(?1,'spatial',?2,?2,1) ON CONFLICT DO NOTHING"
                    ),
                    params![id, now],
                )?;
            } else {
                tx.execute(
                    &format!("DELETE FROM {table} WHERE {column}=?1 AND capability_id='spatial'"),
                    [id],
                )?;
            }
        }
    }
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1", [now])?;
    // Read before commit: a failed acknowledgement cannot invite a duplicate creation.
    let result = read(&tx)?;
    tx.commit()?;
    Ok(result)
}
