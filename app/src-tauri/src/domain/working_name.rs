//! A validated Project working (display) name.
//!
//! This is presentation state, never identity: two Projects may share a
//! working name, and changing it must never affect a [`super::ProjectId`].

use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::DomainError;

pub const MAX_WORKING_NAME_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkingName(String);

impl WorkingName {
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyWorkingName);
        }
        if trimmed.chars().count() > MAX_WORKING_NAME_LEN {
            return Err(DomainError::WorkingNameTooLong {
                max: MAX_WORKING_NAME_LEN,
            });
        }
        Ok(WorkingName(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for WorkingName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        let name = WorkingName::new("  Tortuga  ").unwrap();
        assert_eq!(name.as_str(), "Tortuga");
    }

    #[test]
    fn rejects_empty_or_blank_names() {
        assert!(matches!(
            WorkingName::new(""),
            Err(DomainError::EmptyWorkingName)
        ));
        assert!(matches!(
            WorkingName::new("   "),
            Err(DomainError::EmptyWorkingName)
        ));
    }

    #[test]
    fn rejects_overly_long_names() {
        let long = "a".repeat(MAX_WORKING_NAME_LEN + 1);
        assert!(matches!(
            WorkingName::new(&long),
            Err(DomainError::WorkingNameTooLong { .. })
        ));
    }
}
