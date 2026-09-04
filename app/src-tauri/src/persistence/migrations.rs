//! Versioned, repeatable embedded migrations.
//!
//! Migrations are tracked with SQLite's built-in `user_version` pragma. A
//! database whose `user_version` is newer than [`CURRENT_SCHEMA_VERSION`]
//! must never be opened for writing by this build -- it may belong to a
//! newer Worldcrafter version.
//!
//! Older supported schemas are upgraded as one atomic chain after the caller
//! acquires the Project lock and creates a validated external recovery point.
//! The database may therefore be ahead of the manifest only when manifest
//! publication was interrupted; the open path republishes that cache safely.

use rusqlite::{Connection, Transaction};

use super::error::PersistenceError;

/// The newest schema version this build knows how to read and write.
pub const CURRENT_SCHEMA_VERSION: i64 = 2;

/// Ordered (version, sql) pairs. Each migration is applied at most once and
/// migrations must be applied in order starting just above the database's
/// current `user_version`.
type MigrationHook = for<'connection> fn(&Transaction<'connection>) -> Result<(), PersistenceError>;

struct Migration {
    version: i64,
    sql: &'static str,
    after_sql: Option<MigrationHook>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("migrations/0001_init.sql"),
        after_sql: None,
    },
    Migration {
        version: 2,
        sql: include_str!("migrations/0002_project_structure.sql"),
        after_sql: Some(add_uncategorized),
    },
];

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

    let pending: Vec<&Migration> = MIGRATIONS
        .iter()
        .filter(|migration| migration.version > current_version)
        .collect();
    if pending.is_empty() {
        return Ok(());
    }

    apply_pending_chain(conn, current_version, &pending)?;
    Ok(())
}

fn apply_pending_chain(
    conn: &Connection,
    current_version: i64,
    pending: &[&Migration],
) -> Result<(), PersistenceError> {
    let tx = conn.unchecked_transaction()?;
    let mut applied_version = current_version;
    for migration in pending {
        if migration.version != applied_version + 1 {
            return Err(PersistenceError::Other(format!(
                "migration chain is not contiguous after schema version {applied_version}"
            )));
        }
        tx.execute_batch(migration.sql)?;
        if let Some(after_sql) = migration.after_sql {
            after_sql(&tx)?;
        }
        applied_version = migration.version;
    }
    tx.execute(
        "UPDATE project_meta SET schema_version = ?1 WHERE id = 1",
        [applied_version],
    )?;
    tx.pragma_update(None, "user_version", applied_version)?;
    tx.commit()?;
    Ok(())
}

fn add_uncategorized(tx: &Transaction<'_>) -> Result<(), PersistenceError> {
    let now = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO category (
            id, name, is_uncategorized, created_at, updated_at, revision
         ) VALUES (?1, 'Uncategorized', 1, ?2, ?2, 0)",
        rusqlite::params![crate::domain::CategoryId::new().to_string(), now],
    )?;
    Ok(())
}

/// Refuses to open an existing database for writing unless its schema already
/// matches this build exactly.
pub fn require_current_schema(conn: &Connection) -> Result<(), PersistenceError> {
    let current_version = user_version(conn)?;
    if current_version > CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::UnsupportedSchemaVersion {
            found: current_version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    if current_version < CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::MigrationRequired {
            found: current_version,
            supported: CURRENT_SCHEMA_VERSION,
        });
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
    fn a_failing_multi_step_chain_rolls_back_every_pending_migration_and_can_retry() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE project_meta (
                id INTEGER PRIMARY KEY,
                schema_version INTEGER NOT NULL
             );
             INSERT INTO project_meta VALUES (1, 0);",
        )
        .unwrap();
        let first = Migration {
            version: 1,
            sql: "CREATE TABLE migration_probe (id INTEGER PRIMARY KEY);",
            after_sql: None,
        };
        let failing = Migration {
            version: 2,
            sql: "INSERT INTO table_that_does_not_exist VALUES (1);",
            after_sql: None,
        };
        let error = apply_pending_chain(&conn, 0, &[&first, &failing]).unwrap_err();
        assert!(matches!(error, PersistenceError::Sqlite(_)));
        assert_eq!(user_version(&conn).unwrap(), 0);
        assert_eq!(
            conn.query_row(
                "SELECT schema_version FROM project_meta WHERE id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        assert!(!table_exists(&conn, "migration_probe"));

        let second = Migration {
            version: 2,
            sql: "CREATE TABLE migration_probe_two (id INTEGER PRIMARY KEY);",
            after_sql: None,
        };
        apply_pending_chain(&conn, 0, &[&first, &second]).unwrap();
        assert_eq!(user_version(&conn).unwrap(), 2);
        assert!(table_exists(&conn, "migration_probe"));
        assert!(table_exists(&conn, "migration_probe_two"));
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
             )",
            [name],
            |row| row.get(0),
        )
        .unwrap()
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

    #[test]
    fn require_current_schema_refuses_to_auto_migrate_an_older_database() {
        let conn = Connection::open_in_memory().unwrap();
        let err = require_current_schema(&conn).unwrap_err();
        assert!(matches!(
            err,
            PersistenceError::MigrationRequired {
                found: 0,
                supported: CURRENT_SCHEMA_VERSION,
            }
        ));
        assert_eq!(user_version(&conn).unwrap(), 0);
    }

    #[test]
    fn parent_type_cycles_and_cross_category_parents_are_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        migrate(&conn).unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let category_a = crate::domain::CategoryId::new().to_string();
        let category_b = crate::domain::CategoryId::new().to_string();
        conn.execute(
            "INSERT INTO category VALUES (?1, 'A', 0, ?3, ?3, 1), (?2, 'B', 0, ?3, ?3, 1)",
            rusqlite::params![category_a, category_b, now],
        )
        .unwrap();
        let type_a = crate::domain::TypeId::new().to_string();
        let type_b = crate::domain::TypeId::new().to_string();
        conn.execute(
            "INSERT INTO type_def VALUES (?1, ?3, NULL, 'A1', ?5, ?5, 1),
                                         (?2, ?3, ?1, 'A2', ?5, ?5, 1)",
            rusqlite::params![type_a, type_b, category_a, category_b, now],
        )
        .unwrap();
        assert!(conn
            .execute(
                "UPDATE type_def SET parent_type_id = ?1 WHERE id = ?2",
                rusqlite::params![type_b, type_a],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE type_def SET category_id = ?1 WHERE id = ?2",
                rusqlite::params![category_b, type_b],
            )
            .is_err());
    }
}
