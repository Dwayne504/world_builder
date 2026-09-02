//! Durability configuration for a Project's SQLite connection.
//!
//! Per the approved architecture, every Project connection uses WAL
//! journaling, enforced foreign keys, and `synchronous=FULL` so that a
//! commit acknowledgement is a genuine durable-write guarantee -- the basis
//! of the "Saved" contract shown to the user.

use rusqlite::Connection;

use super::error::PersistenceError;

/// A conservative busy timeout so a momentarily busy database (e.g. during
/// a backup snapshot) does not immediately fail a write.
pub const BUSY_TIMEOUT_MS: u32 = 5_000;

pub fn apply(conn: &Connection) -> Result<(), PersistenceError> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS as u64))?;
    Ok(())
}

/// Snapshot of the durability-relevant pragmas, used by tests and
/// diagnostics to verify the configuration actually took effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilityStatus {
    pub journal_mode: String,
    pub foreign_keys: bool,
    pub synchronous: i64,
}

pub fn read_status(conn: &Connection) -> Result<DurabilityStatus, PersistenceError> {
    let journal_mode: String = conn.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
    let foreign_keys: i64 = conn.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
    let synchronous: i64 = conn.pragma_query_value(None, "synchronous", |r| r.get(0))?;
    Ok(DurabilityStatus {
        journal_mode,
        foreign_keys: foreign_keys != 0,
        synchronous,
    })
}
