use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreferencesError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the preferences file is unreadable or corrupt ({0}); it was left untouched")]
    Corrupt(String),
    #[error("could not determine the application configuration directory: {0}")]
    NoConfigDir(String),
}
