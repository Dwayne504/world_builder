//! Domain-level errors: invariant violations that are independent of how or
//! where the data is stored.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("working name must not be empty")]
    EmptyWorkingName,

    #[error("working name is too long (max {max} characters)")]
    WorkingNameTooLong { max: usize },

    #[error("'{0}' is not a valid Project ID")]
    InvalidProjectId(String),

    #[error("'{1}' is not a valid {0} ID")]
    InvalidStructureId(&'static str, String),

    #[error("Category and Type names must not be empty")]
    EmptyDefinitionName,

    #[error("Category and Type names are too long (max {max} characters)")]
    DefinitionNameTooLong { max: usize },
}
