use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::ProjectId;
use crate::package::PackagePaths;
use crate::persistence::lock::LockGuard;
use crate::persistence::ProjectDbWorker;

/// Everything the application layer needs to know about one currently open
/// Project. Owned exclusively by [`AppState`]; nothing outside
/// `application` touches `worker` or `lock` directly.
pub struct OpenProject {
    pub worker: ProjectDbWorker,
    pub paths: PackagePaths,
    pub lock: LockGuard,
}

/// Process-wide registry of open Projects. Held behind a `Mutex` inside
/// Tauri's managed state; the mutex only ever guards short registry
/// operations (insert/remove/lookup), never a blocking SQLite call.
#[derive(Default)]
pub struct AppState {
    pub open_projects: Mutex<HashMap<ProjectId, OpenProject>>,
}

/// A read-only, serializable snapshot of a Project's state, suitable for
/// display and for returning from Tauri commands.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub project_id: ProjectId,
    pub working_name: String,
    pub revision: i64,
    pub package_path: String,
    pub format_version: i64,
    pub schema_version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
