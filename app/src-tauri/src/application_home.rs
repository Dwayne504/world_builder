//! Application-local recent Projects. No Project package is written by this store.
//! SQLite transactions serialize app processes and make interrupted updates recoverable.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use thiserror::Error;

use crate::application::ProjectSummary;
use crate::domain::ProjectId;

const RETENTION: i64 = 3;
const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Error)]
pub enum RecentError {
    #[error("Recent Projects could not be read: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("Recent Projects could not be stored: {0}")]
    Io(#[from] std::io::Error),
    #[error("This version cannot read the Recent Projects list (version {0}).")]
    Unsupported(i64),
    #[error("This Project is no longer in Recent Projects. Browse for it to open it.")]
    Missing,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub project_id: String,
    pub working_name: String,
    pub package_path: String,
    pub last_accessed_at: String,
    pub available: bool,
}

pub struct RecentProjectsStore {
    path: PathBuf,
}

impl RecentProjectsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn connect(&self) -> Result<Connection, RecentError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        match version {
            0 => {
                tx.execute_batch(
                    "CREATE TABLE recent_project (
                        project_id TEXT PRIMARY KEY NOT NULL,
                        working_name TEXT NOT NULL,
                        package_path TEXT NOT NULL,
                        last_accessed_at TEXT NOT NULL,
                        access_order INTEGER NOT NULL
                    ); PRAGMA user_version = 1;",
                )?;
            }
            SCHEMA_VERSION => {}
            other => return Err(RecentError::Unsupported(other)),
        }
        tx.commit()?;
        Ok(connection)
    }

    pub fn list(&self) -> Result<Vec<RecentProject>, RecentError> {
        let connection = self.connect()?;
        let mut query = connection.prepare(
            "SELECT project_id, working_name, package_path, last_accessed_at
             FROM recent_project ORDER BY access_order DESC LIMIT ?1",
        )?;
        let rows = query.query_map([RETENTION], |row| {
            let package_path: String = row.get(2)?;
            // Listing never opens a package or acquires its advisory lock.
            let available = Path::new(&package_path).is_dir();
            Ok(RecentProject {
                project_id: row.get(0)?,
                working_name: row.get(1)?,
                package_path,
                last_accessed_at: row.get(3)?,
                available,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn remember(&self, summary: &ProjectSummary) -> Result<(), RecentError> {
        let path = std::fs::canonicalize(&summary.package_path)?;
        let mut connection = self.connect()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO recent_project
             (project_id, working_name, package_path, last_accessed_at, access_order)
             VALUES (?1, ?2, ?3, ?4, (SELECT COALESCE(MAX(access_order), 0) + 1 FROM recent_project))
             ON CONFLICT(project_id) DO UPDATE SET working_name=excluded.working_name,
             package_path=excluded.package_path, last_accessed_at=excluded.last_accessed_at,
             access_order=excluded.access_order",
            params![summary.project_id.to_string(), summary.working_name,
                path.to_string_lossy(), chrono::Utc::now().to_rfc3339()],
        )?;
        tx.execute(
            "DELETE FROM recent_project WHERE project_id NOT IN
             (SELECT project_id FROM recent_project ORDER BY access_order DESC LIMIT ?1)",
            [RETENTION],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn path_for(&self, id: ProjectId) -> Result<PathBuf, RecentError> {
        self.list()?
            .into_iter()
            .find(|p| p.project_id == id.to_string())
            .map(|p| PathBuf::from(p.package_path))
            .ok_or(RecentError::Missing)
    }

    pub fn forget(&self, id: ProjectId) -> Result<(), RecentError> {
        self.connect()?.execute(
            "DELETE FROM recent_project WHERE project_id=?1",
            [id.to_string()],
        )?;
        Ok(())
    }
}
