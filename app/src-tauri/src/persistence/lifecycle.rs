use super::PersistenceError;
use crate::domain::{lifecycle::*, require_definition_name, CategoryId};
use rusqlite::{params, Connection, TransactionBehavior};

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
pub(super) fn preview(
    conn: &Connection,
    id: CategoryId,
) -> Result<CategoryDeletePreview, PersistenceError> {
    let (name, fallback): (String, bool) = conn.query_row(
        "SELECT name,is_uncategorized FROM category WHERE id=?1",
        [id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if fallback {
        return Err(invalid(
            "Uncategorized is the fallback Category and cannot be deleted",
        ));
    }
    let (entry_count, typed_entry_count) = conn.query_row(
        "SELECT count(*),count(type_id) FROM entry WHERE category_id=?1",
        [id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let mut statement =
        conn.prepare("SELECT name FROM type_def WHERE category_id=?1 ORDER BY name,id")?;
    let type_names = statement
        .query_map([id.to_string()], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    let default_count = conn.query_row("SELECT count(*) FROM field_availability WHERE (provider_kind='category' AND provider_id=?1) OR (provider_kind='type' AND provider_id IN (SELECT id FROM type_def WHERE category_id=?1))", [id.to_string()], |r| r.get(0))?;
    Ok(CategoryDeletePreview {
        global_revision: revision(conn)?,
        name,
        entry_count,
        typed_entry_count,
        type_names,
        default_count,
    })
}
pub(super) fn apply(
    conn: &mut Connection,
    expected: i64,
    command: StructureCommand,
) -> Result<StructureOutcome, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = revision(&tx)?;
    if expected != current {
        return Err(PersistenceError::StaleRevision { expected, current });
    }
    let now = chrono::Utc::now().to_rfc3339();
    match command {
        StructureCommand::RenameType { id, name } => {
            let name = require_definition_name(&name).map_err(invalid)?;
            if tx.execute(
                "UPDATE type_def SET name=?2,revision=revision+1,updated_at=?3 WHERE id=?1",
                params![id.to_string(), name, now],
            )? != 1
            {
                return Err(invalid("Type not found in this Project"));
            }
        }
        StructureCommand::RenameCategory { id, name } => {
            let name = require_definition_name(&name).map_err(invalid)?;
            if tx.execute("UPDATE category SET name=?2,revision=revision+1,updated_at=?3 WHERE id=?1 AND is_uncategorized=0", params![id.to_string(),name,now])? != 1 { return Err(invalid("Choose an author-created Category to rename")); }
        }
        StructureCommand::DeleteCategory {
            id,
            destination_id,
            remove_types,
        } => {
            let review = preview(&tx, id)?;
            if id == destination_id
                || !tx
                    .prepare("SELECT 1 FROM category WHERE id=?1")?
                    .exists([destination_id.to_string()])?
            {
                return Err(invalid("Choose another Category for these Entries"));
            }
            if !review.type_names.is_empty() && !remove_types {
                return Err(invalid(
                    "Review and confirm removal of this Category's Types first",
                ));
            }
            // Category-local Types cannot follow an Entry into an unrelated Category.
            // This operation requires explicit review; filled-in Fields and all links stay.
            tx.execute("UPDATE entry SET category_id=?2,type_id=NULL,revision=revision+1,updated_at=?3 WHERE category_id=?1",params![id.to_string(),destination_id.to_string(),now])?;
            tx.execute("DELETE FROM field_availability WHERE (provider_kind='category' AND provider_id=?1) OR (provider_kind='type' AND provider_id IN (SELECT id FROM type_def WHERE category_id=?1))",[id.to_string()])?;
            tx.execute(
                "DELETE FROM category_capability_default WHERE category_id=?1",
                [id.to_string()],
            )?;
            tx.execute("DELETE FROM type_capability_default WHERE type_id IN (SELECT id FROM type_def WHERE category_id=?1)",[id.to_string()])?;
            tx.execute(
                "UPDATE type_def SET parent_type_id=NULL WHERE category_id=?1",
                [id.to_string()],
            )?;
            tx.execute(
                "DELETE FROM type_def WHERE category_id=?1",
                [id.to_string()],
            )?;
            tx.execute("DELETE FROM category WHERE id=?1", [id.to_string()])?;
        }
        StructureCommand::SetEntryState { id, state } => {
            if tx.execute("UPDATE record_identity SET workspace_state=?2,lifecycle_changed_at=?3 WHERE record_id=?1 AND kind='entry'",params![id.to_string(),state.as_str(),now])? != 1 { return Err(invalid("Entry not found in this Project")); }
            tx.execute(
                "UPDATE entry SET revision=revision+1,updated_at=?2 WHERE id=?1",
                params![id.to_string(), now],
            )?;
        }
    }
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1",[now])?;
    let result = StructureOutcome {
        global_revision: revision(&tx)?,
        backup_path: None,
    };
    tx.commit()?;
    Ok(result)
}
