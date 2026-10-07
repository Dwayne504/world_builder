//! Wire-format DTOs for the Tauri boundary. Kept separate from
//! `application::ProjectSummary` so the application layer's internal
//! representation can evolve without silently changing the IPC contract.

use serde::Serialize;

use crate::application::{AppError, ProjectSummary};
use crate::domain::{Category, Entry, TypeDef};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfoDto {
    pub version: String,
    pub supported_schema_version: i64,
    pub supported_format_version: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummaryDto {
    pub recent_projects_warning: Option<String>,
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
            recent_projects_warning: None,
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
    pub workspace_state: String,
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
            workspace_state: value.workspace_state,
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
    pub automatic_backups_enabled: bool,
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
            automatic_backups_enabled: prefs.automatic_backups_enabled,
            default_projects_dir: prefs.default_projects_dir.map(|p| p.display().to_string()),
            default_projects_dir_exists: projects_exists,
            default_backups_dir: prefs.default_backups_dir.map(|p| p.display().to_string()),
            default_backups_dir_exists: backups_exists,
        }
    }
}

impl From<AppError> for AppErrorDto {
    fn from(e: AppError) -> Self {
        // Keep compatibility checks fail-closed, but explain what the author
        // should do instead of exposing a database implementation diagnostic.
        let message = match &e {
            AppError::Persistence(
                crate::persistence::PersistenceError::UnsupportedSchemaVersion { found, supported },
            ) => newer_project_message("storage", *found, *supported),
            AppError::Package(crate::package::PackageError::UnsupportedFormatVersion {
                found,
                supported,
            }) => newer_project_message("package", *found, *supported),
            _ => e.to_string(),
        };
        AppErrorDto {
            kind: e.kind().to_string(),
            message,
        }
    }
}

fn newer_project_message(component: &str, found: i64, supported: i64) -> String {
    format!("This Project needs a newer Worldcrafter build. Open it with the build that last saved it or a newer one. It has not been opened for editing. (Project {component} version: {found}; this build supports: {supported}.)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_project_errors_explain_the_safe_next_step_without_changing_the_kind() {
        let schema: AppErrorDto = AppError::Persistence(
            crate::persistence::PersistenceError::UnsupportedSchemaVersion {
                found: 12,
                supported: 11,
            },
        )
        .into();
        assert_eq!(schema.kind, "unsupported_schema_version");
        assert!(schema.message.contains("newer Worldcrafter build"));
        assert!(schema.message.contains("has not been opened for editing"));
        assert!(schema.message.contains("storage version: 12"));
        assert!(schema.message.contains("supports: 11"));
        let package: AppErrorDto =
            AppError::Package(crate::package::PackageError::UnsupportedFormatVersion {
                found: 2,
                supported: 1,
            })
            .into();
        assert_eq!(package.kind, "unsupported_format_version");
        assert!(package.message.contains("package version: 2"));
        assert!(package.message.contains("supports: 1"));
    }

    #[test]
    fn other_errors_keep_their_original_diagnostic() {
        let error = AppError::Persistence(crate::persistence::PersistenceError::Other(
            "disk write failed".into(),
        ));
        let original = error.to_string();
        let dto: AppErrorDto = error.into();
        assert_eq!(dto.kind, "persistence_error");
        assert_eq!(dto.message, original);
    }

    #[test]
    fn about_information_comes_from_the_running_backend() {
        let info = super::super::commands::get_build_info();
        let json = serde_json::to_value(info).unwrap();
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(
            json["supportedSchemaVersion"],
            crate::persistence::migrations::CURRENT_SCHEMA_VERSION
        );
        assert_eq!(
            json["supportedFormatVersion"],
            crate::package::FORMAT_VERSION
        );
    }
}
