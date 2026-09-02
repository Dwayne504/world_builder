//! `ProjectDbWorker`: the single owner of a Project's `rusqlite` connection.
//!
//! Each open Project has exactly one worker, running on its own dedicated
//! blocking OS thread. All SQLite access for that Project goes through a
//! serialized command queue processed by that thread -- nothing else in the
//! process ever touches the connection. This gives a concrete
//! thread/ownership model instead of sharing a connection across tasks.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, TransactionBehavior};

use crate::domain::ProjectId;

use super::error::PersistenceError;
use super::{migrations, pragmas};

/// Metadata supplied when creating a brand-new Project database. Absent
/// when opening an existing one.
#[derive(Debug, Clone)]
pub struct InitialProjectMeta {
    pub working_name: String,
    pub format_version: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectMetaSnapshot {
    pub project_id: ProjectId,
    pub working_name: String,
    pub format_version: i64,
    pub schema_version: i64,
    pub last_committed_revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenameOutcome {
    pub committed_revision: i64,
    pub updated_at: DateTime<Utc>,
}

type Reply<T> = Sender<Result<T, PersistenceError>>;

enum Job {
    ReadMeta {
        reply: Reply<ProjectMetaSnapshot>,
    },
    RenameProject {
        expected_revision: i64,
        new_working_name: String,
        reply: Reply<RenameOutcome>,
    },
    /// Runs a consistent SQLite Online Backup of the live database to
    /// `dest_path`, executed on the worker thread so it observes the same
    /// connection every other write goes through.
    BackupTo {
        dest_path: PathBuf,
        reply: Reply<()>,
    },
    Shutdown {
        reply: Reply<()>,
    },
}

/// A handle to a running `ProjectDbWorker`. Cloning is not supported: one
/// worker owns one connection for one open Project.
pub struct ProjectDbWorker {
    sender: Sender<Job>,
    handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for ProjectDbWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectDbWorker").finish_non_exhaustive()
    }
}

impl ProjectDbWorker {
    /// Spawns the worker thread, opens `db_path`, applies durability
    /// pragmas, migrates the schema, and validates/initializes
    /// `project_meta`. Blocks the caller until startup succeeds or fails so
    /// callers can treat this like any other fallible open.
    pub fn spawn(
        db_path: PathBuf,
        expected_project_id: ProjectId,
        initial: Option<InitialProjectMeta>,
    ) -> Result<Self, PersistenceError> {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), PersistenceError>>();

        let handle = thread::Builder::new()
            .name(format!("project-db-{expected_project_id}"))
            .spawn(move || {
                let outcome = Self::open_and_prepare(&db_path, expected_project_id, initial);
                let conn = match outcome {
                    Ok(conn) => {
                        let _ = ready_tx.send(Ok(()));
                        conn
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                Self::run(conn, job_rx);
            })
            .expect("failed to spawn Project database worker thread");

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(ProjectDbWorker {
                sender: job_tx,
                handle: Some(handle),
            }),
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(e)
            }
            Err(_) => {
                let _ = handle.join();
                Err(PersistenceError::WorkerShutDown)
            }
        }
    }

    fn open_and_prepare(
        db_path: &PathBuf,
        expected_project_id: ProjectId,
        initial: Option<InitialProjectMeta>,
    ) -> Result<Connection, PersistenceError> {
        let conn = Connection::open(db_path)?;
        pragmas::apply(&conn)?;
        migrations::migrate(&conn)?;

        let existing: Option<(String, i64)> = conn
            .query_row(
                "SELECT project_id, last_committed_revision FROM project_meta WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();

        match (existing, initial) {
            (Some((db_project_id, _)), _) => {
                let db_project_id = ProjectId::parse(&db_project_id)
                    .map_err(|e| PersistenceError::Other(e.to_string()))?;
                if db_project_id != expected_project_id {
                    return Err(PersistenceError::ProjectIdMismatch {
                        manifest: expected_project_id,
                        database: db_project_id,
                    });
                }
                Ok(conn)
            }
            (None, Some(init)) => {
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "INSERT INTO project_meta (
                        id, project_id, format_version, schema_version, working_name,
                        last_committed_revision, created_at, updated_at
                    ) VALUES (1, ?1, ?2, ?3, ?4, 0, ?5, ?5)",
                    rusqlite::params![
                        expected_project_id.to_string(),
                        init.format_version,
                        migrations::CURRENT_SCHEMA_VERSION,
                        init.working_name,
                        now,
                    ],
                )?;
                Ok(conn)
            }
            (None, None) => Err(PersistenceError::MissingProjectMeta),
        }
    }

    fn run(conn: Connection, jobs: Receiver<Job>) {
        let mut conn = conn;
        for job in jobs {
            match job {
                Job::ReadMeta { reply } => {
                    let _ = reply.send(read_meta(&conn));
                }
                Job::RenameProject {
                    expected_revision,
                    new_working_name,
                    reply,
                } => {
                    let _ = reply.send(rename_project(
                        &mut conn,
                        expected_revision,
                        &new_working_name,
                    ));
                }
                Job::BackupTo { dest_path, reply } => {
                    let _ = reply.send(backup_to(&conn, &dest_path));
                }
                Job::Shutdown { reply } => {
                    let _ = reply.send(Ok(()));
                    break;
                }
            }
        }
    }

    fn call<T>(&self, build: impl FnOnce(Reply<T>) -> Job) -> Result<T, PersistenceError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.sender
            .send(build(reply_tx))
            .map_err(|_| PersistenceError::WorkerShutDown)?;
        reply_rx
            .recv()
            .map_err(|_| PersistenceError::WorkerShutDown)?
    }

    pub fn read_meta(&self) -> Result<ProjectMetaSnapshot, PersistenceError> {
        self.call(|reply| Job::ReadMeta { reply })
    }

    pub fn rename_project(
        &self,
        expected_revision: i64,
        new_working_name: String,
    ) -> Result<RenameOutcome, PersistenceError> {
        self.call(|reply| Job::RenameProject {
            expected_revision,
            new_working_name,
            reply,
        })
    }

    /// Runs a consistent SQLite Online Backup snapshot to `dest_path`.
    pub fn backup_to(&self, dest_path: PathBuf) -> Result<(), PersistenceError> {
        self.call(|reply| Job::BackupTo { dest_path, reply })
    }

    /// Requests a controlled shutdown: any already-queued jobs are drained
    /// (in order) before the worker thread exits, so pending writes are
    /// never silently discarded.
    pub fn shutdown(mut self) -> Result<(), PersistenceError> {
        let result = self.call(|reply| Job::Shutdown { reply });
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        result
    }
}

impl Drop for ProjectDbWorker {
    fn drop(&mut self) {
        // Best-effort: an explicit `shutdown()` is the supported path and
        // already consumes `self`; this only guards against a caller that
        // drops the worker without closing it cleanly.
        let (reply_tx, _reply_rx) = mpsc::channel();
        let _ = self.sender.send(Job::Shutdown { reply: reply_tx });
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_meta(conn: &Connection) -> Result<ProjectMetaSnapshot, PersistenceError> {
    conn.query_row(
        "SELECT project_id, working_name, format_version, schema_version,
                last_committed_revision, created_at, updated_at
         FROM project_meta WHERE id = 1",
        [],
        |r| {
            let project_id: String = r.get(0)?;
            let created_at: String = r.get(5)?;
            let updated_at: String = r.get(6)?;
            Ok((
                project_id,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                created_at,
                updated_at,
            ))
        },
    )
    .map_err(PersistenceError::from)
    .and_then(
        |(
            project_id,
            working_name,
            format_version,
            schema_version,
            revision,
            created_at,
            updated_at,
        )| {
            Ok(ProjectMetaSnapshot {
                project_id: ProjectId::parse(&project_id)
                    .map_err(|e| PersistenceError::Other(e.to_string()))?,
                working_name,
                format_version,
                schema_version,
                last_committed_revision: revision,
                created_at: parse_timestamp(&created_at)?,
                updated_at: parse_timestamp(&updated_at)?,
            })
        },
    )
}

fn parse_timestamp(raw: &str) -> Result<DateTime<Utc>, PersistenceError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| PersistenceError::Other(e.to_string()))
}

/// Renames the Project's working name inside a single `BEGIN IMMEDIATE`
/// transaction, rejecting stale `expected_revision`s so a client editing an
/// out-of-date snapshot never silently overwrites a newer commit.
fn rename_project(
    conn: &mut Connection,
    expected_revision: i64,
    new_working_name: &str,
) -> Result<RenameOutcome, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let current_revision: i64 = tx.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id = 1",
        [],
        |r| r.get(0),
    )?;
    if current_revision != expected_revision {
        // Rolls back automatically when `tx` is dropped.
        return Err(PersistenceError::StaleRevision {
            expected: expected_revision,
            current: current_revision,
        });
    }

    let new_revision = current_revision + 1;
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let changed = tx.execute(
        "UPDATE project_meta SET working_name = ?1, last_committed_revision = ?2, updated_at = ?3 WHERE id = 1",
        rusqlite::params![new_working_name, new_revision, now_str],
    )?;
    if changed != 1 {
        return Err(PersistenceError::MissingProjectMeta);
    }
    tx.commit()?;

    Ok(RenameOutcome {
        committed_revision: new_revision,
        updated_at: now,
    })
}

fn backup_to(conn: &Connection, dest_path: &std::path::Path) -> Result<(), PersistenceError> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut dest = Connection::open(dest_path)?;
    {
        let backup = rusqlite::backup::Backup::new(conn, &mut dest)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(50), None)?;
    }
    // Validate the snapshot before declaring success.
    let integrity: String = dest.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(PersistenceError::Other(format!(
            "backup snapshot failed integrity_check: {integrity}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn spawn_fresh(dir: &std::path::Path, project_id: ProjectId) -> ProjectDbWorker {
        ProjectDbWorker::spawn(
            dir.join("project.sqlite"),
            project_id,
            Some(InitialProjectMeta {
                working_name: "Tortuga".to_string(),
                format_version: 1,
            }),
        )
        .unwrap()
    }

    #[test]
    fn fresh_database_gets_matching_project_id_and_zero_revision() {
        let dir = tempdir().unwrap();
        let project_id = ProjectId::new();
        let worker = spawn_fresh(dir.path(), project_id);
        let meta = worker.read_meta().unwrap();
        assert_eq!(meta.project_id, project_id);
        assert_eq!(meta.last_committed_revision, 0);
        assert_eq!(meta.working_name, "Tortuga");
        worker.shutdown().unwrap();
    }

    #[test]
    fn durability_pragmas_are_wal_foreign_keys_full_sync() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("project.sqlite");
        let conn = Connection::open(&db_path).unwrap();
        pragmas::apply(&conn).unwrap();
        let status = pragmas::read_status(&conn).unwrap();
        assert_eq!(status.journal_mode.to_lowercase(), "wal");
        assert!(status.foreign_keys);
        // SQLite's synchronous pragma: 0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA.
        assert!(status.synchronous >= 2);
    }

    #[test]
    fn rename_commits_and_id_is_unchanged() {
        let dir = tempdir().unwrap();
        let project_id = ProjectId::new();
        let worker = spawn_fresh(dir.path(), project_id);

        let outcome = worker
            .rename_project(0, "Tortuga Renamed".to_string())
            .unwrap();
        assert_eq!(outcome.committed_revision, 1);

        let meta = worker.read_meta().unwrap();
        assert_eq!(
            meta.project_id, project_id,
            "renaming must not change the Project ID"
        );
        assert_eq!(meta.working_name, "Tortuga Renamed");
        assert_eq!(meta.last_committed_revision, 1);
        worker.shutdown().unwrap();
    }

    #[test]
    fn stale_expected_revision_is_rejected_and_state_is_unchanged() {
        let dir = tempdir().unwrap();
        let worker = spawn_fresh(dir.path(), ProjectId::new());
        worker
            .rename_project(0, "First Rename".to_string())
            .unwrap();

        // Retry with the now-stale expected_revision=0.
        let err = worker
            .rename_project(0, "Stale Rename".to_string())
            .unwrap_err();
        assert!(matches!(
            err,
            PersistenceError::StaleRevision {
                expected: 0,
                current: 1
            }
        ));

        // The failed transaction must leave the previously committed state
        // untouched.
        let meta = worker.read_meta().unwrap();
        assert_eq!(meta.working_name, "First Rename");
        assert_eq!(meta.last_committed_revision, 1);
        worker.shutdown().unwrap();
    }

    #[test]
    fn reopening_the_same_database_retains_the_committed_name() {
        let dir = tempdir().unwrap();
        let project_id = ProjectId::new();
        {
            let worker = spawn_fresh(dir.path(), project_id);
            worker
                .rename_project(0, "Persisted Name".to_string())
                .unwrap();
            worker.shutdown().unwrap();
        }

        // Reopen: no InitialProjectMeta this time, since the row already
        // exists.
        let worker =
            ProjectDbWorker::spawn(dir.path().join("project.sqlite"), project_id, None).unwrap();
        let meta = worker.read_meta().unwrap();
        assert_eq!(meta.working_name, "Persisted Name");
        assert_eq!(meta.last_committed_revision, 1);
        worker.shutdown().unwrap();
    }

    #[test]
    fn opening_with_mismatched_project_id_is_rejected() {
        let dir = tempdir().unwrap();
        let created_id = ProjectId::new();
        {
            let worker = spawn_fresh(dir.path(), created_id);
            worker.shutdown().unwrap();
        }

        let wrong_id = ProjectId::new();
        let err =
            ProjectDbWorker::spawn(dir.path().join("project.sqlite"), wrong_id, None).unwrap_err();
        assert!(matches!(err, PersistenceError::ProjectIdMismatch { .. }));
    }
}
