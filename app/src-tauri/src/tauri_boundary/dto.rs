//! Wire-format DTOs for the Tauri boundary. Kept separate from
//! `application::ProjectSummary` so the application layer's internal
//! representation can evolve without silently changing the IPC contract.

use serde::Serialize;

use crate::application::{AppError, ProjectSummary};
use crate::domain::{Category, Entry, TypeDef};

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

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub is_uncategorized: bool,
    pub revision: i64,
    pub global_revision: i64,
}

impl From<Category> for CategoryDto {
    fn from(value: Category) -> Self {
        Self {
            id: value.id.to_string(),
            name: value.name,
            is_uncategorized: value.is_uncategorized,
            revision: value.revision,
            global_revision: value.global_revision,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TypeDto {
    pub id: String,
    pub category_id: String,
    pub parent_type_id: Option<String>,
    pub name: String,
    pub revision: i64,
    pub global_revision: i64,
}

impl From<TypeDef> for TypeDto {
    fn from(value: TypeDef) -> Self {
        Self {
            id: value.id.to_string(),
            category_id: value.category_id.to_string(),
            parent_type_id: value.parent_type_id.map(|id| id.to_string()),
            name: value.name,
            revision: value.revision,
            global_revision: value.global_revision,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryDto {
    pub id: String,
    pub category_id: String,
    pub type_id: Option<String>,
    pub authored_name: Option<String>,
    pub display_name: String,
    pub revision: i64,
    pub global_revision: i64,
}

impl From<Entry> for EntryDto {
    fn from(value: Entry) -> Self {
        let display_name = value.display_name().to_string();
        Self {
            id: value.id.to_string(),
            category_id: value.category_id.to_string(),
            type_id: value.type_id.map(|id| id.to_string()),
            authored_name: value.authored_name,
            display_name,
            revision: value.revision,
            global_revision: value.global_revision,
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

/// Application-level preference state, plus liveness flags so the frontend
/// can warn about a configured directory that has since been moved or
/// become inaccessible without guessing at a silent fallback.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesDto {
    pub default_projects_dir: Option<String>,
    pub default_projects_dir_exists: bool,
    pub default_backups_dir: Option<String>,
    pub default_backups_dir_exists: bool,
}

impl From<crate::preferences::AppPreferences> for PreferencesDto {
    fn from(prefs: crate::preferences::AppPreferences) -> Self {
        let projects_exists = prefs
            .default_projects_dir
            .as_deref()
            .is_some_and(crate::preferences::directory_is_usable);
        let backups_exists = prefs
            .default_backups_dir
            .as_deref()
            .is_some_and(crate::preferences::directory_is_usable);
        PreferencesDto {
            default_projects_dir: prefs.default_projects_dir.map(|p| p.display().to_string()),
            default_projects_dir_exists: projects_exists,
            default_backups_dir: prefs.default_backups_dir.map(|p| p.display().to_string()),
            default_backups_dir_exists: backups_exists,
        }
    }
}

impl From<AppError> for AppErrorDto {
    fn from(e: AppError) -> Self {
        AppErrorDto {
            kind: e.kind().to_string(),
            message: e.to_string(),
        }
    }
}
