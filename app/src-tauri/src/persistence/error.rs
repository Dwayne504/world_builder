use thiserror::Error;

use crate::domain::ProjectId;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error(
        "database schema version {found} is newer than the {supported} this build supports; \
         refusing to open it for writing"
    )]
    UnsupportedSchemaVersion { found: i64, supported: i64 },

    #[error(
        "database schema version {found} is older than the writable schema version {supported}; \
         refusing to auto-migrate until coordinated manifest publication exists"
    )]
    MigrationRequired { found: i64, supported: i64 },

    #[error(
        "manifest Project ID {manifest} does not match the database Project ID {database}; \
         refusing to open"
    )]
    ProjectIdMismatch {
        manifest: ProjectId,
        database: ProjectId,
    },

    #[error("database is missing its project_meta row and no initial metadata was supplied")]
    MissingProjectMeta,

    #[error("expected revision {expected} is stale; the current committed revision is {current}")]
    StaleRevision { expected: i64, current: i64 },

    #[error("the Project worker has already shut down")]
    WorkerShutDown,

    #[error("{0}")]
    Other(String),
}
