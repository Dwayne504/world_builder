use thiserror::Error;

use crate::domain::ProjectId;

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Package(#[from] crate::package::PackageError),

    #[error(transparent)]
    Persistence(#[from] crate::persistence::PersistenceError),

    #[error(transparent)]
    Domain(#[from] crate::domain::DomainError),

    #[error("backup snapshot at '{0}' failed SQLite integrity_check")]
    CorruptSnapshot(String),

    #[error(
        "backup manifest Project ID {manifest} does not match its database Project ID {database}"
    )]
    IdentityMismatch {
        manifest: ProjectId,
        database: ProjectId,
    },

    #[error("'{0}' is not a valid Worldcrafter backup")]
    NotABackup(String),

    #[error("unsafe backup or restore path: '{0}'")]
    UnsafePath(String),
}
