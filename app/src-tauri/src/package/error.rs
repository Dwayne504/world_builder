use thiserror::Error;

use crate::domain::ProjectId;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("manifest is not valid JSON: {0}")]
    InvalidManifest(String),

    #[error(
        "'{0}' does not look like a Worldcrafter package \
         (missing manifest.json or data/project.sqlite)"
    )]
    NotAPackage(String),

    #[error("package format version {found} is newer than the {supported} this build supports")]
    UnsupportedFormatVersion { found: i64, supported: i64 },

    #[error("a Project package already exists at '{0}'")]
    AlreadyExists(String),

    #[error("manifest Project ID {0} is invalid: {1}")]
    InvalidProjectId(ProjectId, String),
}
