//! Application layer: named use-case commands.
//!
//! Application commands own transaction/use-case boundaries and map
//! domain/persistence/package failures into a single [`error::AppError`].
//! Nothing above this layer (the Tauri boundary, the React UI) is allowed
//! to talk to SQLite or the package filesystem directly.

pub mod error;
pub mod service;
pub mod state;

pub use error::AppError;
pub use service::ProjectService;
pub use state::{AppState, ProjectSummary};
