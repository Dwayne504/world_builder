//! Domain layer: stable identities and invariants.
//!
//! This module owns product invariants that must hold regardless of how the
//! data is persisted or presented. It has no dependency on Tauri, React,
//! SQLite, or the filesystem.

pub mod error;
pub mod ids;
pub mod structure;
pub mod working_name;

pub use error::DomainError;
pub use ids::ProjectId;
pub use structure::{
    authored_name, require_definition_name, Category, CategoryId, Entry, EntryId, TypeDef, TypeId,
};
pub use working_name::WorkingName;
