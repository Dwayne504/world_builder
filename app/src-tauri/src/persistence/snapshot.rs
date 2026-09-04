use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use super::PersistenceError;

pub fn backup_connection(source: &Connection, destination: &Path) -> Result<(), PersistenceError> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut dest = Connection::open(destination)?;
    {
        let backup = rusqlite::backup::Backup::new(source, &mut dest)?;
        backup.run_to_completion(5, Duration::from_millis(50), None)?;
    }
    let integrity: String = dest.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(PersistenceError::Other(format!(
            "backup snapshot failed integrity_check: {integrity}"
        )));
    }
    Ok(())
}
