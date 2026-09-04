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
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior};

use crate::domain::{
    authored_name, Category, CategoryId, Entry, EntryId, ProjectId, TypeDef, TypeId,
};

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

#[derive(Debug, Clone, PartialEq)]
pub struct ExistingProjectPreflight {
    pub schema_version: i64,
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
    ListCategories {
        reply: Reply<Vec<Category>>,
    },
    CreateCategory {
        id: CategoryId,
        name: String,
        reply: Reply<Category>,
    },
    ListTypes {
        category_id: CategoryId,
        reply: Reply<Vec<TypeDef>>,
    },
    CreateType {
        id: TypeId,
        category_id: CategoryId,
        parent_type_id: Option<TypeId>,
        name: String,
        reply: Reply<TypeDef>,
    },
    ListEntries {
        reply: Reply<Vec<Entry>>,
    },
    CreateEntry {
        id: EntryId,
        category_id: Option<CategoryId>,
        type_id: Option<TypeId>,
        authored_name: Option<String>,
        reply: Reply<Entry>,
    },
    GetEntry {
        id: EntryId,
        reply: Reply<Entry>,
    },
    UpdateEntryName {
        id: EntryId,
        expected_revision: i64,
        authored_name: Option<String>,
        reply: Reply<Entry>,
    },
    ChangeEntryStructure {
        id: EntryId,
        expected_revision: i64,
        category_id: CategoryId,
        type_id: Option<TypeId>,
        reply: Reply<Entry>,
    },
    /// Runs a consistent SQLite Online Backup of the live database to
    /// `dest_path`, executed on the worker thread so it observes the same
    /// connection every other write goes through.
    BackupTo {
        dest_path: PathBuf,
        reply: Reply<ProjectMetaSnapshot>,
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
    /// Non-destructively checks whether an existing Project database can be
    /// opened by this build. This never creates a database, acquires locks, or
    /// flips durability pragmas.
    pub fn preflight_existing(
        db_path: PathBuf,
        expected_project_id: ProjectId,
    ) -> Result<ExistingProjectPreflight, PersistenceError> {
        let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let user_version = migrations::user_version(&conn)?;
        if user_version > migrations::CURRENT_SCHEMA_VERSION {
            return Err(PersistenceError::UnsupportedSchemaVersion {
                found: user_version,
                supported: migrations::CURRENT_SCHEMA_VERSION,
            });
        }
        let Some((database_id, _, format_version, schema_version)) =
            read_existing_project_meta(&conn)?
        else {
            return Err(PersistenceError::MissingProjectMeta);
        };
        let database_id = ProjectId::parse(&database_id)
            .map_err(|error| PersistenceError::Other(error.to_string()))?;
        if database_id != expected_project_id {
            return Err(PersistenceError::ProjectIdMismatch {
                manifest: expected_project_id,
                database: database_id,
            });
        }
        if format_version != crate::package::FORMAT_VERSION || schema_version != user_version {
            return Err(PersistenceError::Other(
                "project metadata version does not match the database schema".to_string(),
            ));
        }
        Ok(ExistingProjectPreflight { schema_version })
    }

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
        let is_initializing = initial.is_some();
        let conn = Connection::open_with_flags(
            db_path,
            if is_initializing {
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
            } else {
                OpenFlags::SQLITE_OPEN_READ_WRITE
            },
        )?;
        if is_initializing {
            pragmas::apply(&conn)?;
            migrations::migrate(&conn)?;
        } else {
            pragmas::apply(&conn)?;
            migrations::migrate(&conn)?;
        }

        let existing = read_existing_project_meta(&conn)?;

        match (existing, initial) {
            (Some(_), _) => {
                validate_existing_project_meta(&conn, expected_project_id)?;
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
                Job::ListCategories { reply } => {
                    let _ = reply.send(list_categories(&conn));
                }
                Job::CreateCategory { id, name, reply } => {
                    let _ = reply.send(create_category(&mut conn, id, &name));
                }
                Job::ListTypes { category_id, reply } => {
                    let _ = reply.send(list_types(&conn, category_id));
                }
                Job::CreateType {
                    id,
                    category_id,
                    parent_type_id,
                    name,
                    reply,
                } => {
                    let _ = reply.send(create_type(
                        &mut conn,
                        id,
                        category_id,
                        parent_type_id,
                        &name,
                    ));
                }
                Job::ListEntries { reply } => {
                    let _ = reply.send(list_entries(&conn));
                }
                Job::CreateEntry {
                    id,
                    category_id,
                    type_id,
                    authored_name,
                    reply,
                } => {
                    let _ = reply.send(create_entry(
                        &mut conn,
                        id,
                        category_id,
                        type_id,
                        authored_name,
                    ));
                }
                Job::GetEntry { id, reply } => {
                    let _ = reply.send(get_entry(&conn, id));
                }
                Job::UpdateEntryName {
                    id,
                    expected_revision,
                    authored_name,
                    reply,
                } => {
                    let _ = reply.send(update_entry_name(
                        &mut conn,
                        id,
                        expected_revision,
                        authored_name,
                    ));
                }
                Job::ChangeEntryStructure {
                    id,
                    expected_revision,
                    category_id,
                    type_id,
                    reply,
                } => {
                    let _ = reply.send(change_entry_structure(
                        &mut conn,
                        id,
                        expected_revision,
                        category_id,
                        type_id,
                    ));
                }
                Job::BackupTo { dest_path, reply } => {
                    let _ =
                        reply.send(backup_to(&conn, &dest_path).and_then(|()| read_meta(&conn)));
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

    pub fn list_categories(&self) -> Result<Vec<Category>, PersistenceError> {
        self.call(|reply| Job::ListCategories { reply })
    }

    pub fn create_category(
        &self,
        id: CategoryId,
        name: String,
    ) -> Result<Category, PersistenceError> {
        self.call(|reply| Job::CreateCategory { id, name, reply })
    }

    pub fn list_types(&self, category_id: CategoryId) -> Result<Vec<TypeDef>, PersistenceError> {
        self.call(|reply| Job::ListTypes { category_id, reply })
    }

    pub fn create_type(
        &self,
        id: TypeId,
        category_id: CategoryId,
        parent_type_id: Option<TypeId>,
        name: String,
    ) -> Result<TypeDef, PersistenceError> {
        self.call(|reply| Job::CreateType {
            id,
            category_id,
            parent_type_id,
            name,
            reply,
        })
    }

    pub fn list_entries(&self) -> Result<Vec<Entry>, PersistenceError> {
        self.call(|reply| Job::ListEntries { reply })
    }

    pub fn create_entry(
        &self,
        id: EntryId,
        category_id: Option<CategoryId>,
        type_id: Option<TypeId>,
        name: Option<String>,
    ) -> Result<Entry, PersistenceError> {
        self.call(|reply| Job::CreateEntry {
            id,
            category_id,
            type_id,
            authored_name: authored_name(name),
            reply,
        })
    }

    pub fn get_entry(&self, id: EntryId) -> Result<Entry, PersistenceError> {
        self.call(|reply| Job::GetEntry { id, reply })
    }

    pub fn update_entry_name(
        &self,
        id: EntryId,
        expected_revision: i64,
        name: Option<String>,
    ) -> Result<Entry, PersistenceError> {
        self.call(|reply| Job::UpdateEntryName {
            id,
            expected_revision,
            authored_name: authored_name(name),
            reply,
        })
    }

    pub fn change_entry_structure(
        &self,
        id: EntryId,
        expected_revision: i64,
        category_id: CategoryId,
        type_id: Option<TypeId>,
    ) -> Result<Entry, PersistenceError> {
        self.call(|reply| Job::ChangeEntryStructure {
            id,
            expected_revision,
            category_id,
            type_id,
            reply,
        })
    }

    /// Runs a consistent SQLite Online Backup snapshot to `dest_path`.
    pub fn backup_to(&self, dest_path: PathBuf) -> Result<ProjectMetaSnapshot, PersistenceError> {
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

fn read_existing_project_meta(
    conn: &Connection,
) -> Result<Option<(String, i64, i64, i64)>, PersistenceError> {
    conn.query_row(
        "SELECT project_id, last_committed_revision, format_version, schema_version FROM project_meta WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .optional()
    .map_err(PersistenceError::from)
}

fn validate_existing_project_meta(
    conn: &Connection,
    expected_project_id: ProjectId,
) -> Result<(), PersistenceError> {
    let Some((db_project_id, _revision, format_version, schema_version)) =
        read_existing_project_meta(conn)?
    else {
        return Err(PersistenceError::MissingProjectMeta);
    };

    let db_project_id =
        ProjectId::parse(&db_project_id).map_err(|e| PersistenceError::Other(e.to_string()))?;
    if db_project_id != expected_project_id {
        return Err(PersistenceError::ProjectIdMismatch {
            manifest: expected_project_id,
            database: db_project_id,
        });
    }
    if format_version != crate::package::FORMAT_VERSION
        || schema_version != migrations::user_version(conn)?
        || schema_version != migrations::CURRENT_SCHEMA_VERSION
    {
        return Err(PersistenceError::Other(
            "project metadata version does not match supported database schema".to_string(),
        ));
    }
    Ok(())
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
    super::snapshot::backup_connection(conn, dest_path)
}

fn next_global_revision(
    tx: &rusqlite::Transaction<'_>,
    now: &str,
) -> Result<i64, PersistenceError> {
    let current: i64 = tx.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id = 1",
        [],
        |r| r.get(0),
    )?;
    let next = current + 1;
    tx.execute(
        "UPDATE project_meta SET last_committed_revision = ?1, updated_at = ?2 WHERE id = 1",
        rusqlite::params![next, now],
    )?;
    Ok(next)
}

fn list_categories(conn: &Connection) -> Result<Vec<Category>, PersistenceError> {
    let mut statement = conn.prepare(
        "SELECT id, name, is_uncategorized, revision
             FROM category ORDER BY is_uncategorized DESC, name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |row| {
        let id: String = row.get(0)?;
        Ok((id, row.get(1)?, row.get::<_, bool>(2)?, row.get(3)?))
    })?;
    rows.map(|row| {
        let (id, name, is_uncategorized, revision) = row?;
        Ok(Category {
            id: CategoryId::parse(&id).map_err(|e| PersistenceError::Other(e.to_string()))?,
            name,
            is_uncategorized,
            revision,
        })
    })
    .collect()
}

fn create_category(
    conn: &mut Connection,
    id: CategoryId,
    name: &str,
) -> Result<Category, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let now = Utc::now().to_rfc3339();
    next_global_revision(&tx, &now)?;
    tx.execute(
        "INSERT INTO category (id, name, is_uncategorized, created_at, updated_at, revision)
             VALUES (?1, ?2, 0, ?3, ?3, 1)",
        rusqlite::params![id.to_string(), name, now],
    )?;
    tx.commit()?;
    Ok(Category {
        id,
        name: name.to_string(),
        is_uncategorized: false,
        revision: 1,
    })
}

fn list_types(
    conn: &Connection,
    category_id: CategoryId,
) -> Result<Vec<TypeDef>, PersistenceError> {
    let mut statement = conn.prepare(
        "SELECT id, category_id, parent_type_id, name, revision
             FROM type_def WHERE category_id = ?1 ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([category_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    rows.map(|row| {
        let (id, category, parent, name, revision) = row?;
        Ok(TypeDef {
            id: TypeId::parse(&id).map_err(|e| PersistenceError::Other(e.to_string()))?,
            category_id: CategoryId::parse(&category)
                .map_err(|e| PersistenceError::Other(e.to_string()))?,
            parent_type_id: parent
                .map(|id| TypeId::parse(&id))
                .transpose()
                .map_err(|e| PersistenceError::Other(e.to_string()))?,
            name,
            revision,
        })
    })
    .collect()
}

fn create_type(
    conn: &mut Connection,
    id: TypeId,
    category_id: CategoryId,
    parent_type_id: Option<TypeId>,
    name: &str,
) -> Result<TypeDef, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let now = Utc::now().to_rfc3339();
    next_global_revision(&tx, &now)?;
    tx.execute(
        "INSERT INTO type_def (
                id, category_id, parent_type_id, name, created_at, updated_at, revision
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)",
        rusqlite::params![
            id.to_string(),
            category_id.to_string(),
            parent_type_id.map(|id| id.to_string()),
            name,
            now
        ],
    )?;
    tx.commit()?;
    Ok(TypeDef {
        id,
        category_id,
        parent_type_id,
        name: name.to_string(),
        revision: 1,
    })
}

fn list_entries(conn: &Connection) -> Result<Vec<Entry>, PersistenceError> {
    let global_revision: i64 = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let mut statement = conn.prepare(
        "SELECT id, category_id, type_id, authored_name, revision
             FROM entry ORDER BY COALESCE(authored_name, '') COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    rows.map(|row| entry_from_row(row?, global_revision))
        .collect()
}

fn get_entry(conn: &Connection, id: EntryId) -> Result<Entry, PersistenceError> {
    let global_revision: i64 = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let row = conn
        .query_row(
            "SELECT id, category_id, type_id, authored_name, revision
                 FROM entry WHERE id = ?1",
            [id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| PersistenceError::Other(format!("Entry {id} was not found")))?;
    entry_from_row(row, global_revision)
}

fn entry_from_row(
    row: (String, String, Option<String>, Option<String>, i64),
    global_revision: i64,
) -> Result<Entry, PersistenceError> {
    Ok(Entry {
        id: EntryId::parse(&row.0).map_err(|e| PersistenceError::Other(e.to_string()))?,
        category_id: CategoryId::parse(&row.1)
            .map_err(|e| PersistenceError::Other(e.to_string()))?,
        type_id: row
            .2
            .map(|id| TypeId::parse(&id))
            .transpose()
            .map_err(|e| PersistenceError::Other(e.to_string()))?,
        authored_name: row.3,
        revision: row.4,
        global_revision,
    })
}

fn create_entry(
    conn: &mut Connection,
    id: EntryId,
    category_id: Option<CategoryId>,
    type_id: Option<TypeId>,
    name: Option<String>,
) -> Result<Entry, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let category_id = match category_id {
        Some(id) => id,
        None => {
            let raw: String = tx.query_row(
                "SELECT id FROM category WHERE is_uncategorized = 1",
                [],
                |row| row.get(0),
            )?;
            CategoryId::parse(&raw).map_err(|e| PersistenceError::Other(e.to_string()))?
        }
    };
    validate_type_category(&tx, category_id, type_id)?;
    let now = Utc::now().to_rfc3339();
    let global_revision = next_global_revision(&tx, &now)?;
    tx.execute(
        "INSERT INTO record_identity (
                record_id, kind, workspace_state, lifecycle_changed_at, created_at
             ) VALUES (?1, 'entry', 'active', ?2, ?2)",
        rusqlite::params![id.to_string(), now],
    )?;
    tx.execute(
        "INSERT INTO entry (
                id, category_id, type_id, authored_name, created_at, updated_at, revision
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)",
        rusqlite::params![
            id.to_string(),
            category_id.to_string(),
            type_id.map(|id| id.to_string()),
            name,
            now
        ],
    )?;
    tx.commit()?;
    Ok(Entry {
        id,
        category_id,
        type_id,
        authored_name: name,
        revision: 1,
        global_revision,
    })
}

fn validate_type_category(
    tx: &rusqlite::Transaction<'_>,
    category_id: CategoryId,
    type_id: Option<TypeId>,
) -> Result<(), PersistenceError> {
    if let Some(type_id) = type_id {
        let matches: bool = tx.query_row(
            "SELECT EXISTS(
                    SELECT 1 FROM type_def WHERE id = ?1 AND category_id = ?2
                 )",
            rusqlite::params![type_id.to_string(), category_id.to_string()],
            |row| row.get(0),
        )?;
        if !matches {
            return Err(PersistenceError::Other(
                "the selected Type does not belong to the selected Category".to_string(),
            ));
        }
    }
    Ok(())
}

fn checked_entry_revision(
    tx: &rusqlite::Transaction<'_>,
    id: EntryId,
    expected: i64,
) -> Result<(), PersistenceError> {
    let current: i64 = tx
        .query_row(
            "SELECT revision FROM entry WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| PersistenceError::Other(format!("Entry {id} was not found")))?;
    if current != expected {
        return Err(PersistenceError::StaleRevision { expected, current });
    }
    Ok(())
}

fn update_entry_name(
    conn: &mut Connection,
    id: EntryId,
    expected_revision: i64,
    name: Option<String>,
) -> Result<Entry, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    checked_entry_revision(&tx, id, expected_revision)?;
    let now = Utc::now().to_rfc3339();
    let global_revision = next_global_revision(&tx, &now)?;
    tx.execute(
        "UPDATE entry
             SET authored_name = ?1, updated_at = ?2, revision = revision + 1
             WHERE id = ?3",
        rusqlite::params![name, now, id.to_string()],
    )?;
    tx.commit()?;
    let mut result = get_entry(conn, id)?;
    result.global_revision = global_revision;
    Ok(result)
}

fn change_entry_structure(
    conn: &mut Connection,
    id: EntryId,
    expected_revision: i64,
    category_id: CategoryId,
    type_id: Option<TypeId>,
) -> Result<Entry, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    checked_entry_revision(&tx, id, expected_revision)?;
    validate_type_category(&tx, category_id, type_id)?;
    let now = Utc::now().to_rfc3339();
    let global_revision = next_global_revision(&tx, &now)?;
    tx.execute(
        "UPDATE entry
             SET category_id = ?1, type_id = ?2, updated_at = ?3, revision = revision + 1
             WHERE id = ?4",
        rusqlite::params![
            category_id.to_string(),
            type_id.map(|id| id.to_string()),
            now,
            id.to_string()
        ],
    )?;
    tx.commit()?;
    let mut result = get_entry(conn, id)?;
    result.global_revision = global_revision;
    Ok(result)
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

    #[test]
    fn preflight_existing_refuses_to_create_a_missing_database() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("missing.sqlite");

        let err =
            ProjectDbWorker::preflight_existing(db_path.clone(), ProjectId::new()).unwrap_err();
        assert!(matches!(err, PersistenceError::Sqlite(_)));
        assert!(!db_path.exists());
    }

    #[test]
    fn schema_v1_existing_database_migrates_without_changing_project_data() {
        let dir = tempdir().unwrap();
        let project_id = ProjectId::new();
        {
            let worker = spawn_fresh(dir.path(), project_id);
            worker.shutdown().unwrap();
        }

        let db_path = dir.path().join("project.sqlite");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "DROP TRIGGER entry_type_category_update;
             DROP TRIGGER entry_type_category_insert;
             DROP TABLE entry;
             DROP TABLE record_identity;
             DROP TRIGGER type_parent_cycle_update;
             DROP TRIGGER type_parent_valid_insert;
             DROP TABLE type_def;
             DROP TABLE category;",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute(
            "UPDATE project_meta SET schema_version = 1 WHERE id = 1",
            [],
        )
        .unwrap();
        drop(conn);

        let preflight = ProjectDbWorker::preflight_existing(db_path.clone(), project_id).unwrap();
        assert_eq!(preflight.schema_version, 1);
        let worker = ProjectDbWorker::spawn(db_path, project_id, None).unwrap();
        assert_eq!(worker.read_meta().unwrap().project_id, project_id);
        let categories = worker.list_categories().unwrap();
        assert_eq!(
            categories
                .iter()
                .filter(|category| category.is_uncategorized)
                .count(),
            1
        );
        worker.shutdown().unwrap();
    }

    #[test]
    fn entries_preserve_identity_and_reject_incompatible_types_and_stale_writes() {
        let dir = tempdir().unwrap();
        let worker = spawn_fresh(dir.path(), ProjectId::new());
        let characters = worker
            .create_category(CategoryId::new(), "Characters".into())
            .unwrap();
        let places = worker
            .create_category(CategoryId::new(), "Places".into())
            .unwrap();
        let human = worker
            .create_type(TypeId::new(), characters.id, None, "Human".into())
            .unwrap();
        let entry = worker
            .create_entry(
                EntryId::new(),
                Some(characters.id),
                Some(human.id),
                Some("Thron".into()),
            )
            .unwrap();
        let renamed = worker
            .update_entry_name(entry.id, entry.revision, Some("Thron II".into()))
            .unwrap();
        assert_eq!(renamed.id, entry.id);

        let incompatible =
            worker.change_entry_structure(entry.id, renamed.revision, places.id, Some(human.id));
        assert!(incompatible.is_err());
        let unchanged = worker.get_entry(entry.id).unwrap();
        assert_eq!(unchanged.category_id, characters.id);
        assert_eq!(unchanged.type_id, Some(human.id));

        let moved = worker
            .change_entry_structure(entry.id, renamed.revision, places.id, None)
            .unwrap();
        assert_eq!(moved.id, entry.id);
        assert_eq!(moved.type_id, None);
        assert!(matches!(
            worker.update_entry_name(entry.id, renamed.revision, Some("stale".into())),
            Err(PersistenceError::StaleRevision { .. })
        ));
        worker.shutdown().unwrap();
    }

    #[test]
    fn entry_without_category_is_valid_and_uses_uncategorized() {
        let dir = tempdir().unwrap();
        let worker = spawn_fresh(dir.path(), ProjectId::new());
        let uncategorized = worker
            .list_categories()
            .unwrap()
            .into_iter()
            .find(|category| category.is_uncategorized)
            .unwrap();
        let entry = worker
            .create_entry(EntryId::new(), None, None, None)
            .unwrap();
        assert_eq!(entry.category_id, uncategorized.id);
        assert_eq!(entry.authored_name, None);
        worker.shutdown().unwrap();
    }
}
