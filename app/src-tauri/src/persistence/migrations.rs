//! Versioned, repeatable embedded migrations.
//!
//! Migrations are tracked with SQLite's built-in `user_version` pragma. A
//! database whose `user_version` is newer than [`CURRENT_SCHEMA_VERSION`]
//! must never be opened for writing by this build -- it may belong to a
//! newer Worldcrafter version.

use rusqlite::Connection;

use super::error::PersistenceError;

/// The newest schema version this build knows how to read and write.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

/// Ordered (version, sql) pairs. Each migration is applied at most once and
/// migrations must be applied in order starting just above the database's
/// current `user_version`.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("migrations/0001_init.sql"))];

/// Applies any migrations newer than the database's current version.
///
/// Returns [`PersistenceError::UnsupportedSchemaVersion`] without modifying
/// the database if the database is already newer than this build supports.
pub fn migrate(conn: &Connection) -> Result<(), PersistenceError> {
    let current_version = user_version(conn)?;
    if current_version > CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::UnsupportedSchemaVersion {
            found: current_version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }

    for (version, sql) in MIGRATIONS {
        if *version > current_version {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()?;
        }
    }
    Ok(())
}

pub fn user_version(conn: &Connection) -> Result<i64, PersistenceError> {
    Ok(conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_migrates_to_current_version() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
        // project_meta must now exist and be queryable.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_meta", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn refuses_to_touch_a_newer_unsupported_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
            .unwrap();
        let err = migrate(&conn).unwrap_err();
        assert!(matches!(
            err,
            PersistenceError::UnsupportedSchemaVersion { .. }
        ));
    }
}
