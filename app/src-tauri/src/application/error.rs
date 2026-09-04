use thiserror::Error;

use crate::domain::{DomainError, ProjectId};
use crate::package::PackageError;
use crate::persistence::lock::LockError;
use crate::persistence::PersistenceError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Package(#[from] PackageError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Backup(#[from] crate::backup_recovery::BackupError),

    #[error("{0}")]
    Lock(#[from] LockError),

    #[error("no open Project with ID {0}")]
    ProjectNotOpen(ProjectId),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    /// A short machine-readable category the frontend can branch on
    /// (e.g. to offer a "retry" affordance for `revision_conflict`,
    /// vs. surfacing `lock_held` with the current owner's details).
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Domain(_) => "invalid_input",
            AppError::Package(PackageError::AlreadyExists(_)) => "already_exists",
            AppError::Package(PackageError::UnsupportedFormatVersion { .. }) => {
                "unsupported_format_version"
            }
            AppError::Package(_) => "invalid_package",
            AppError::Persistence(PersistenceError::StaleRevision { .. }) => "revision_conflict",
            AppError::Persistence(PersistenceError::ProjectIdMismatch { .. }) => {
                "identity_mismatch"
            }
            AppError::Persistence(PersistenceError::UnsupportedSchemaVersion { .. }) => {
                "unsupported_schema_version"
            }
            AppError::Persistence(PersistenceError::MigrationRequired { .. }) => {
                "migration_required"
            }
            AppError::Persistence(_) => "persistence_error",
            AppError::Backup(_) => "backup_error",
            AppError::Lock(LockError::Held { .. }) => "lock_held",
            AppError::Lock(_) => "lock_error",
            AppError::ProjectNotOpen(_) => "project_not_open",
            AppError::Io(_) => "io_error",
        }
    }
}
