//! Typed Tauri commands. Each command validates nothing itself beyond
//! parsing its DTO arguments -- all product/invariant validation happens in
//! `application`/`domain` -- and immediately delegates to
//! [`crate::application::ProjectService`].

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::application::{AppState, ProjectService, ProjectSummary};
use crate::application_home::{RecentError, RecentProject, RecentProjectsStore};
use crate::domain::{CategoryId, EntryId, ProjectId, TypeId};
use crate::package::layout;
use crate::preferences::{self, PreferencesError, PreferencesStore};

use super::dto::{AppErrorDto, CategoryDto, EntryDto, PreferencesDto, ProjectSummaryDto, TypeDto};

fn parse_project_id(raw: &str) -> Result<ProjectId, AppErrorDto> {
    ProjectId::parse(raw).map_err(|e| AppErrorDto {
        kind: "invalid_input".to_string(),
        message: e.to_string(),
    })
}

fn invalid_input(message: impl ToString) -> AppErrorDto {
    AppErrorDto {
        kind: "invalid_input".to_string(),
        message: message.to_string(),
    }
}

#[tauri::command]
pub fn automatic_backup_directory(
    app: AppHandle,
    preferences: State<'_, PreferencesStore>,
) -> Result<String, AppErrorDto> {
    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|e| invalid_input(e.to_string()))?;
    Ok(preferences
        .automatic_backup_root(&app_data)?
        .display()
        .to_string())
}

#[tauri::command]
pub fn delete_entry_field(
    app: AppHandle,
    state: State<'_, AppState>,
    preferences: State<'_, PreferencesStore>,
    project_id: String,
    entry_id: String,
    field_id: String,
    expected_revision: i64,
) -> Result<crate::domain::fields::EntryFieldDeleteOutcome, AppErrorDto> {
    let backup_dir = automatic_backup_directory(app, preferences)?;
    ProjectService::delete_entry_field(
        &state,
        parse_project_id(&project_id)?,
        EntryId::parse(&entry_id).map_err(invalid_input)?,
        crate::domain::structure::FieldId::parse(&field_id).map_err(invalid_input)?,
        expected_revision,
        &PathBuf::from(backup_dir),
    )
    .map_err(Into::into)
}

#[tauri::command]
pub fn preview_field_merge(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
    target_id: String,
) -> Result<crate::domain::fields::FieldMergePreview, AppErrorDto> {
    ProjectService::preview_field_merge(
        &state,
        parse_project_id(&project_id)?,
        crate::domain::structure::FieldId::parse(&source_id).map_err(invalid_input)?,
        crate::domain::structure::FieldId::parse(&target_id).map_err(invalid_input)?,
    )
    .map_err(Into::into)
}
#[tauri::command]
pub fn merge_fields(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
    target_id: String,
    expected_revision: i64,
    backup_dir: String,
) -> Result<crate::domain::fields::FieldMergeOutcome, AppErrorDto> {
    preferences::validate_directory(&PathBuf::from(&backup_dir))?;
    ProjectService::merge_fields(
        &state,
        parse_project_id(&project_id)?,
        crate::domain::structure::FieldId::parse(&source_id).map_err(invalid_input)?,
        crate::domain::structure::FieldId::parse(&target_id).map_err(invalid_input)?,
        expected_revision,
        &PathBuf::from(backup_dir),
    )
    .map_err(Into::into)
}

#[tauri::command]
pub fn read_field_catalog(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<crate::domain::fields::FieldCatalog, AppErrorDto> {
    ProjectService::read_field_catalog(&state, parse_project_id(&project_id)?).map_err(Into::into)
}
#[tauri::command]
pub fn apply_template_fields(
    state: State<'_, AppState>,
    project_id: String,
    expected_revision: i64,
    command: crate::domain::fields::FieldCommand,
) -> Result<crate::domain::fields::FieldCatalog, AppErrorDto> {
    ProjectService::apply_template_fields(
        &state,
        parse_project_id(&project_id)?,
        expected_revision,
        command,
    )
    .map_err(Into::into)
}

#[tauri::command]
pub fn read_relationships(
    state: State<'_, AppState>,
    project_id: String,
    entry_id: String,
) -> Result<crate::domain::relationships::EntryRelationships, AppErrorDto> {
    ProjectService::read_relationships(
        &state,
        parse_project_id(&project_id)?,
        EntryId::parse(&entry_id).map_err(invalid_input)?,
    )
    .map_err(Into::into)
}
#[tauri::command]
pub fn apply_relationships(
    state: State<'_, AppState>,
    project_id: String,
    entry_id: String,
    expected_revision: i64,
    command: crate::domain::relationships::RelationshipCommand,
) -> Result<crate::domain::relationships::EntryRelationships, AppErrorDto> {
    ProjectService::apply_relationships(
        &state,
        parse_project_id(&project_id)?,
        EntryId::parse(&entry_id).map_err(invalid_input)?,
        expected_revision,
        command,
    )
    .map_err(Into::into)
}

#[tauri::command]
pub fn read_fields(
    state: State<'_, AppState>,
    project_id: String,
    entry_id: String,
) -> Result<crate::domain::fields::EntryFields, AppErrorDto> {
    ProjectService::read_fields(
        &state,
        parse_project_id(&project_id)?,
        EntryId::parse(&entry_id).map_err(invalid_input)?,
    )
    .map_err(Into::into)
}
#[tauri::command]
pub fn apply_fields(
    state: State<'_, AppState>,
    project_id: String,
    entry_id: String,
    expected_revision: i64,
    command: crate::domain::fields::FieldCommand,
) -> Result<crate::domain::fields::EntryFields, AppErrorDto> {
    ProjectService::apply_fields(
        &state,
        parse_project_id(&project_id)?,
        EntryId::parse(&entry_id).map_err(invalid_input)?,
        expected_revision,
        command,
    )
    .map_err(Into::into)
}

impl From<PreferencesError> for AppErrorDto {
    fn from(error: PreferencesError) -> Self {
        let kind = match &error {
            PreferencesError::Io(_) => "io_error",
            PreferencesError::Corrupt(_) => "preferences_corrupt",
            PreferencesError::UnsupportedVersion { .. } => "unsupported_preferences_version",
            PreferencesError::NoConfigDir(_) => "preferences_unavailable",
            PreferencesError::InvalidDirectory(_) => "invalid_directory",
        };
        AppErrorDto {
            kind: kind.to_string(),
            message: error.to_string(),
        }
    }
}

/// The single on-disk location for application-level preferences: the OS
/// application-config directory, entirely outside every `.wcproj`
/// package and never treated as Project data.
pub(crate) fn preferences_path(app: &AppHandle) -> Result<PathBuf, AppErrorDto> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| PreferencesError::NoConfigDir(e.to_string()))?;
    Ok(dir.join(preferences::PREFERENCES_FILE))
}

impl From<RecentError> for AppErrorDto {
    fn from(error: RecentError) -> Self {
        Self {
            kind: "recent_projects_unavailable".into(),
            message: error.to_string(),
        }
    }
}

// A convenience-list failure must not turn a successful Project commit/open into
// an apparent failure. Return its separate warning with the acknowledged result.
fn remember_project(store: &RecentProjectsStore, summary: ProjectSummary) -> ProjectSummaryDto {
    let warning = store.remember(&summary).err().map(|e| e.to_string());
    let mut dto: ProjectSummaryDto = summary.into();
    dto.recent_projects_warning = warning;
    dto
}

#[tauri::command]
pub fn list_recent_projects(
    recent: State<'_, RecentProjectsStore>,
) -> Result<Vec<RecentProject>, AppErrorDto> {
    recent.list().map_err(Into::into)
}

#[tauri::command]
pub fn forget_recent_project(
    recent: State<'_, RecentProjectsStore>,
    project_id: String,
) -> Result<(), AppErrorDto> {
    recent
        .forget(parse_project_id(&project_id)?)
        .map_err(Into::into)
}

#[tauri::command]
pub fn open_recent_project(
    state: State<'_, AppState>,
    recent: State<'_, RecentProjectsStore>,
    project_id: String,
    relocated_path: Option<String>,
    force_stale_lock_recovery: bool,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    let id = parse_project_id(&project_id)?;
    let saved_path = recent.path_for(id)?;
    let path = relocated_path.map(PathBuf::from).unwrap_or(saved_path);
    ProjectService::open_expected_project(&state, &path, force_stale_lock_recovery, Some(id))
        .map(|summary| remember_project(&recent, summary))
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    recent: State<'_, RecentProjectsStore>,
    base_dir: String,
    working_name: String,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    preferences::validate_directory(std::path::Path::new(&base_dir))?;
    ProjectService::create_project(&state, &PathBuf::from(base_dir), &working_name)
        .map(|summary| remember_project(&recent, summary))
        .map_err(Into::into)
}

#[tauri::command]
pub fn open_project(
    state: State<'_, AppState>,
    recent: State<'_, RecentProjectsStore>,
    package_path: String,
    force_stale_lock_recovery: bool,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    ProjectService::open_project(
        &state,
        &PathBuf::from(package_path),
        force_stale_lock_recovery,
    )
    .map(|summary| remember_project(&recent, summary))
    .map_err(Into::into)
}

#[tauri::command]
pub fn rename_project(
    state: State<'_, AppState>,
    recent: State<'_, RecentProjectsStore>,
    project_id: String,
    new_name: String,
    expected_revision: i64,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    let id = parse_project_id(&project_id)?;
    ProjectService::rename_project(&state, id, &new_name, expected_revision)
        .map(|summary| remember_project(&recent, summary))
        .map_err(Into::into)
}

#[tauri::command]
pub fn close_project(state: State<'_, AppState>, project_id: String) -> Result<(), AppErrorDto> {
    let id = parse_project_id(&project_id)?;
    ProjectService::close_project(&state, id).map_err(Into::into)
}

#[tauri::command]
pub fn get_project_summary(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    let id = parse_project_id(&project_id)?;
    ProjectService::get_summary(&state, id)
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_backup(
    state: State<'_, AppState>,
    project_id: String,
    backup_dir: String,
) -> Result<String, AppErrorDto> {
    let id = parse_project_id(&project_id)?;
    preferences::validate_directory(std::path::Path::new(&backup_dir))?;
    ProjectService::create_backup(&state, id, &PathBuf::from(backup_dir))
        .map(|p| p.display().to_string())
        .map_err(Into::into)
}

#[tauri::command]
pub fn restore_backup_as_copy(
    state: State<'_, AppState>,
    recent: State<'_, RecentProjectsStore>,
    backup_path: String,
    destination_dir: String,
    new_working_name: Option<String>,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    preferences::validate_directory(std::path::Path::new(&destination_dir))?;
    ProjectService::restore_backup_as_copy(
        &state,
        &PathBuf::from(backup_path),
        &PathBuf::from(destination_dir),
        new_working_name.as_deref(),
    )
    .map(|summary| remember_project(&recent, summary))
    .map_err(Into::into)
}

#[tauri::command]
pub fn list_categories(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<CategoryDto>, AppErrorDto> {
    ProjectService::list_categories(&state, parse_project_id(&project_id)?)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_category(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<CategoryDto, AppErrorDto> {
    ProjectService::create_category(&state, parse_project_id(&project_id)?, &name)
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_types(
    state: State<'_, AppState>,
    project_id: String,
    category_id: String,
) -> Result<Vec<TypeDto>, AppErrorDto> {
    let category_id = CategoryId::parse(&category_id).map_err(invalid_input)?;
    ProjectService::list_types(&state, parse_project_id(&project_id)?, category_id)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_type(
    state: State<'_, AppState>,
    project_id: String,
    category_id: String,
    parent_type_id: Option<String>,
    name: String,
) -> Result<TypeDto, AppErrorDto> {
    let category_id = CategoryId::parse(&category_id).map_err(invalid_input)?;
    let parent_type_id = parent_type_id
        .map(|id| TypeId::parse(&id))
        .transpose()
        .map_err(invalid_input)?;
    ProjectService::create_type(
        &state,
        parse_project_id(&project_id)?,
        category_id,
        parent_type_id,
        &name,
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[tauri::command]
pub fn list_entries(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<EntryDto>, AppErrorDto> {
    ProjectService::list_entries(&state, parse_project_id(&project_id)?)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_entry(
    state: State<'_, AppState>,
    project_id: String,
    category_id: Option<String>,
    type_id: Option<String>,
    authored_name: Option<String>,
) -> Result<EntryDto, AppErrorDto> {
    let category_id = category_id
        .map(|id| CategoryId::parse(&id))
        .transpose()
        .map_err(invalid_input)?;
    let type_id = type_id
        .map(|id| TypeId::parse(&id))
        .transpose()
        .map_err(invalid_input)?;
    ProjectService::create_entry(
        &state,
        parse_project_id(&project_id)?,
        category_id,
        type_id,
        authored_name,
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[tauri::command]
pub fn get_entry(
    state: State<'_, AppState>,
    project_id: String,
    entry_id: String,
) -> Result<EntryDto, AppErrorDto> {
    let entry_id = EntryId::parse(&entry_id).map_err(invalid_input)?;
    ProjectService::get_entry(&state, parse_project_id(&project_id)?, entry_id)
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn update_entry_name(
    state: State<'_, AppState>,
    project_id: String,
    entry_id: String,
    authored_name: Option<String>,
    expected_revision: i64,
) -> Result<EntryDto, AppErrorDto> {
    let entry_id = EntryId::parse(&entry_id).map_err(invalid_input)?;
    ProjectService::update_entry_name(
        &state,
        parse_project_id(&project_id)?,
        entry_id,
        expected_revision,
        authored_name,
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[tauri::command]
pub fn change_entry_structure(
    state: State<'_, AppState>,
    project_id: String,
    entry_id: String,
    category_id: String,
    type_id: Option<String>,
    expected_revision: i64,
) -> Result<EntryDto, AppErrorDto> {
    let entry_id = EntryId::parse(&entry_id).map_err(invalid_input)?;
    let category_id = CategoryId::parse(&category_id).map_err(invalid_input)?;
    let type_id = type_id
        .map(|id| TypeId::parse(&id))
        .transpose()
        .map_err(invalid_input)?;
    ProjectService::change_entry_structure(
        &state,
        parse_project_id(&project_id)?,
        entry_id,
        expected_revision,
        category_id,
        type_id,
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[tauri::command]
pub fn get_appearance(
    store: State<'_, PreferencesStore>,
) -> Result<preferences::Appearance, AppErrorDto> {
    Ok(store.load()?.appearance)
}

#[tauri::command]
pub fn set_appearance(
    store: State<'_, PreferencesStore>,
    appearance: preferences::Appearance,
) -> Result<preferences::Appearance, AppErrorDto> {
    Ok(store
        .update(|prefs| prefs.appearance = appearance)?
        .appearance)
}

#[tauri::command]
pub fn get_preferences(store: State<'_, PreferencesStore>) -> Result<PreferencesDto, AppErrorDto> {
    Ok(store.load()?.into())
}

#[tauri::command]
pub fn set_default_projects_dir(
    store: State<'_, PreferencesStore>,
    directory: Option<String>,
) -> Result<PreferencesDto, AppErrorDto> {
    let directory = directory.filter(|d| !d.is_empty()).map(PathBuf::from);
    if let Some(path) = &directory {
        preferences::validate_directory(path)?;
    }
    let prefs = store.update(|prefs| prefs.default_projects_dir = directory)?;
    Ok(prefs.into())
}

#[tauri::command]
pub fn set_default_backups_dir(
    store: State<'_, PreferencesStore>,
    directory: Option<String>,
) -> Result<PreferencesDto, AppErrorDto> {
    let directory = directory.filter(|d| !d.is_empty()).map(PathBuf::from);
    if let Some(path) = &directory {
        preferences::validate_directory(path)?;
    }
    let prefs = store.update(|prefs| prefs.default_backups_dir = directory)?;
    Ok(prefs.into())
}

/// Explicit, user-initiated recovery from a corrupt preferences file. Never invoked
/// automatically; the prior file is preserved under a diagnostic filename
/// by `preferences::reset` before defaults are written.
#[tauri::command]
pub fn reset_preferences(
    store: State<'_, PreferencesStore>,
) -> Result<PreferencesDto, AppErrorDto> {
    Ok(store.reset()?.into())
}

/// Previews the exact package path `create_project` would use for
/// `working_name` under `base_dir`, using the same authoritative
/// sanitization -- so the UI never maintains a second, potentially
/// diverging sanitizer.
#[tauri::command]
pub fn preview_package_path(base_dir: String, working_name: String) -> String {
    layout::single_candidate_package_path(&PathBuf::from(base_dir), &working_name)
        .display()
        .to_string()
}

/// Shows a native folder picker, optionally starting in `default_path`.
/// Returns `None` when the user cancels the dialog; this is never treated
/// as an error.
#[tauri::command]
pub async fn pick_directory(
    app: AppHandle,
    default_path: Option<String>,
) -> Result<Option<String>, AppErrorDto> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut builder = app.dialog().file();
        if let Some(path) = default_path.filter(|p| !p.is_empty()) {
            builder = builder.set_directory(path);
        }
        builder
            .blocking_pick_folder()
            .and_then(|picked| picked.into_path().ok())
            .map(|p| p.display().to_string())
    })
    .await
    .map_err(|error| invalid_input(error.to_string()))
}
