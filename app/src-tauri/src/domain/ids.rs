//! Stable Project identity.
//!
//! A [`ProjectId`] is a UUIDv7 value allocated once when a Project is
//! created. It is immutable for the lifetime of the Project: renaming,
//! reopening, or moving the package must never change it. Restoring a
//! backup as a copy is the one operation that intentionally allocates a
//! *new* `ProjectId` (see `backup_recovery`).
//!
//! Visible names are never identifiers: nothing in this codebase may derive
//! a `ProjectId` from a working name, file name, or path.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(Uuid);

impl ProjectId {
    /// Allocates a new, time-ordered (UUIDv7) Project identity.
    pub fn new() -> Self {
        ProjectId(Uuid::now_v7())
    }

    /// Parses a previously allocated Project ID, e.g. read back from a
    /// manifest or database row.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(value)
            .map(ProjectId)
            .map_err(|_| DomainError::InvalidProjectId(value.to_string()))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_string() {
        let id = ProjectId::new();
        let parsed = ProjectId::parse(&id.to_string()).expect("valid id");
        assert_eq!(id, parsed);
    }

    #[test]
    fn rejects_garbage_input() {
        assert!(ProjectId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn two_new_ids_are_distinct_and_time_ordered() {
        let a = ProjectId::new();
        let b = ProjectId::new();
        assert_ne!(a, b);
        // UUIDv7 is time-ordered, so textual/byte order also orders by creation time.
        assert!(a.as_uuid().as_bytes() <= b.as_uuid().as_bytes());
    }
}
