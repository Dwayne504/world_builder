use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::DomainError;

macro_rules! stable_id {
    ($name:ident, $error:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn parse(value: &str) -> Result<Self, DomainError> {
                Uuid::parse_str(value)
                    .map(Self)
                    .map_err(|_| DomainError::InvalidStructureId($error, value.to_string()))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

stable_id!(CategoryId, "Category");
stable_id!(TypeId, "Type");
stable_id!(EntryId, "Entry");

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub is_uncategorized: bool,
    pub revision: i64,
    pub global_revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeDef {
    pub id: TypeId,
    pub category_id: CategoryId,
    pub parent_type_id: Option<TypeId>,
    pub name: String,
    pub revision: i64,
    pub global_revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub id: EntryId,
    pub category_id: CategoryId,
    pub type_id: Option<TypeId>,
    pub authored_name: Option<String>,
    pub revision: i64,
    pub global_revision: i64,
}

impl Entry {
    pub fn display_name(&self) -> &str {
        self.authored_name.as_deref().unwrap_or("[Unnamed Entry]")
    }
}

pub fn authored_name(value: Option<String>) -> Option<String> {
    value.and_then(|name| {
        let trimmed = name.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

pub fn require_definition_name(value: &str) -> Result<String, DomainError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(DomainError::EmptyDefinitionName);
    }
    if value.chars().count() > 200 {
        return Err(DomainError::DefinitionNameTooLong { max: 200 });
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unnamed_entry_uses_only_a_presentation_fallback() {
        let entry = Entry {
            id: EntryId::new(),
            category_id: CategoryId::new(),
            type_id: None,
            authored_name: authored_name(Some("  ".into())),
            revision: 0,
            global_revision: 0,
        };
        assert_eq!(entry.authored_name, None);
        assert_eq!(entry.display_name(), "[Unnamed Entry]");
    }

    #[test]
    fn names_do_not_determine_identity() {
        let id = EntryId::new();
        let mut entry = Entry {
            id,
            category_id: CategoryId::new(),
            type_id: None,
            authored_name: Some("Thron".into()),
            revision: 0,
            global_revision: 0,
        };
        entry.authored_name = Some("Thron Renamed".into());
        assert_eq!(entry.id, id);
    }
}
