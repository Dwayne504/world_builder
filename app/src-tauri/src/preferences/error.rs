use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreferencesError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the preferences file is unreadable or corrupt ({0}); it was left untouched")]
    Corrupt(String),
    #[error(
        "preferences schema version {found} is not supported by this build (supported: {supported}); the file was left untouched"
    )]
    UnsupportedVersion { found: i64, supported: i64 },
    #[error("could not determine the application configuration directory: {0}")]
    NoConfigDir(String),
    #[error("'{0}' does not exist or is not a directory")]
    InvalidDirectory(String),
}
