//! Tauri boundary: the only layer that talks to Tauri, and the only layer
//! React is allowed to call. Everything here is a typed command (no raw
//! IPC blobs) that delegates immediately to `application::ProjectService`.

pub mod commands;
pub mod dto;

pub use commands::*;
