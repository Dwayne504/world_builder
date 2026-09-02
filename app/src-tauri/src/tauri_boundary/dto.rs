//! Wire-format DTOs for the Tauri boundary. Kept separate from
//! `application::ProjectSummary` so the application layer's internal
//! representation can evolve without silently changing the IPC contract.

use serde::Serialize;

use crate::application::{AppError, ProjectSummary};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummaryDto {
    pub project_id: String,
    pub working_name: String,
    pub revision: i64,
    pub package_path: String,
    pub format_version: i64,
    pub schema_version: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProjectSummary> for ProjectSummaryDto {
    fn from(s: ProjectSummary) -> Self {
        ProjectSummaryDto {
            project_id: s.project_id.to_string(),
            working_name: s.working_name,
            revision: s.revision,
            package_path: s.package_path,
            format_version: s.format_version,
            schema_version: s.schema_version,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

/// A structured, serializable error the frontend can branch on (e.g. to
/// offer "retry" for `revision_conflict`, or show lock ownership details
/// for `lock_held`) instead of matching on opaque message strings.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub kind: String,
    pub message: String,
}

impl From<AppError> for AppErrorDto {
    fn from(e: AppError) -> Self {
        AppErrorDto {
            kind: e.kind().to_string(),
            message: e.to_string(),
        }
    }
}
