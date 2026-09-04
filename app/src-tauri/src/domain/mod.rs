//! Domain layer: stable identities and invariants.
//!
//! This module owns product invariants that must hold regardless of how the
//! data is persisted or presented. It has no dependency on Tauri, React,
//! SQLite, or the filesystem.

pub mod error;
pub mod ids;
pub mod working_name;

pub use error::DomainError;
pub use ids::ProjectId;
pub use working_name::WorkingName;
