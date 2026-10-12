use super::PersistenceError;
use crate::domain::navigation::*;
use rusqlite::{params, Connection, TransactionBehavior};

fn invalid(message: &str) -> PersistenceError {
    PersistenceError::Other(message.into())
}
fn valid_target(conn: &Connection, target: &NavigationTarget) -> Result<(), PersistenceError> {
    if uuid::Uuid::parse_str(&target.record_id).is_err() {
        return Err(invalid("Invalid navigation target"));
    }
    let table = match target.record_kind {
        NavigationKind::Entry => "entry",
        NavigationKind::StoryUnit => "story_unit",
        NavigationKind::TemporalOccurrence => "temporal_occurrence",
    };
    let exists: bool = conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM record_identity i JOIN {table} r ON r.id=i.record_id WHERE i.record_id=?1 AND i.kind=?2)"),
        params![target.record_id, target.record_kind.as_str()], |r| r.get(0),
    )?;
    if !exists {
        return Err(invalid("Record is not available in this Project"));
    }
    Ok(())
}
fn records(conn: &Connection, pins: bool) -> Result<Vec<NavigationRecord>, PersistenceError> {
    let (table, order) = if pins {
        ("project_pin", "pinned_order ASC")
    } else {
        ("project_recent", "access_rank DESC")
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT n.record_kind,n.record_id,i.workspace_state,CASE n.record_kind
         WHEN 'entry' THEN e.authored_name WHEN 'story_unit' THEN s.title
         ELSE COALESCE(NULLIF(TRIM(o.title),''),event.authored_name) END
         FROM {table} n
         LEFT JOIN record_identity i ON i.record_id=n.record_id AND i.kind=n.record_kind
         LEFT JOIN entry e ON n.record_kind='entry' AND e.id=n.record_id
         LEFT JOIN story_unit s ON n.record_kind='story_unit' AND s.id=n.record_id
         LEFT JOIN temporal_occurrence o ON n.record_kind='temporal_occurrence' AND o.id=n.record_id
         LEFT JOIN entry event ON event.id=o.event_entry_id
         ORDER BY n.{order},n.record_id"
    ))?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut result = Vec::new();
    for row in rows {
        let (kind, id, state, label) = row?;
        let (record_kind, fallback) = match kind.as_str() {
            "entry" => (NavigationKind::Entry, "[Unnamed Entry]"),
            "story_unit" => (NavigationKind::StoryUnit, "[Untitled Chapter]"),
            "temporal_occurrence" => (NavigationKind::TemporalOccurrence, "[Untitled occurrence]"),
            _ => continue, // Unknown navigation metadata must not obstruct the workspace.
        };
        result.push(NavigationRecord {
            target: NavigationTarget {
                record_kind,
                record_id: id,
            },
            label: label
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| fallback.into()),
            workspace_state: state.unwrap_or_else(|| "missing".into()),
        });
    }
    Ok(result)
}
pub(super) fn read(conn: &Connection) -> Result<NavigationSnapshot, PersistenceError> {
    Ok(NavigationSnapshot {
        pins: records(conn, true)?,
        recents: records(conn, false)?,
        recent_limit: conn.query_row(
            "SELECT recent_limit FROM project_navigation_settings WHERE id=1",
            [],
            |r| r.get(0),
        )?,
    })
}
pub(super) fn apply(
    conn: &mut Connection,
    command: NavigationCommand,
) -> Result<NavigationSnapshot, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    match command {
        NavigationCommand::Visit { target } => {
            valid_target(&tx, &target)?;
            tx.execute(
                "INSERT INTO project_recent(record_kind,record_id,access_rank)
                VALUES(?1,?2,(SELECT COALESCE(MAX(access_rank),0)+1 FROM project_recent))
                ON CONFLICT(record_kind,record_id) DO UPDATE SET access_rank=excluded.access_rank",
                params![target.record_kind.as_str(), target.record_id],
            )?;
        }
        NavigationCommand::Pin { target, pinned } => {
            if pinned {
                valid_target(&tx, &target)?;
                tx.execute(
                    "INSERT INTO project_pin(record_kind,record_id,pinned_order)
                    VALUES(?1,?2,(SELECT COALESCE(MAX(pinned_order),0)+1 FROM project_pin))
                    ON CONFLICT(record_kind,record_id) DO NOTHING",
                    params![target.record_kind.as_str(), target.record_id],
                )?;
            } else {
                // Removing an unavailable pin never needs to modify its former target.
                tx.execute(
                    "DELETE FROM project_pin WHERE record_kind=?1 AND record_id=?2",
                    params![target.record_kind.as_str(), target.record_id],
                )?;
            }
        }
        NavigationCommand::SetRecentLimit { limit } => {
            if !(1..=100).contains(&limit) {
                return Err(invalid("Choose between 1 and 100 recent records"));
            }
            tx.execute(
                "UPDATE project_navigation_settings SET recent_limit=?1 WHERE id=1",
                [limit],
            )?;
        }
        NavigationCommand::ClearRecents => {
            tx.execute("DELETE FROM project_recent", [])?;
        }
    }
    tx.execute(
        "DELETE FROM project_recent WHERE rowid NOT IN
        (SELECT rowid FROM project_recent ORDER BY access_rank DESC,record_id
        LIMIT (SELECT recent_limit FROM project_navigation_settings WHERE id=1))",
        [],
    )?;
    let result = read(&tx)?;
    // Workspace metadata must not make a concurrent writing draft stale.
    tx.commit()?;
    Ok(result)
}
