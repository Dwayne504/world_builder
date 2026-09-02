//! Persistence layer: SQLite schema, durability configuration, migrations,
//! the dedicated `ProjectDbWorker`, and the Project lock file.
//!
//! This module performs database operations only; it makes no product
//! decisions and is not reachable from `react_ui` except through
//! `application` and `tauri_boundary`.

pub mod error;
pub mod lock;
pub mod migrations;
pub mod pragmas;
pub mod worker;

pub use error::PersistenceError;
pub use worker::{InitialProjectMeta, ProjectDbWorker, ProjectMetaSnapshot, RenameOutcome};
