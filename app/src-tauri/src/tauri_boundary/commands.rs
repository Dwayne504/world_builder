//! Typed Tauri commands. Each command validates nothing itself beyond
//! parsing its DTO arguments -- all product/invariant validation happens in
//! `application`/`domain` -- and immediately delegates to
//! [`crate::application::ProjectService`].

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::application::{AppState, ProjectService};
use crate::domain::{CategoryId, EntryId, ProjectId, TypeId};
use crate::preferences::{self, PreferencesError};

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

impl From<PreferencesError> for AppErrorDto {
    fn from(error: PreferencesError) -> Self {
        let kind = match &error {
            PreferencesError::Io(_) => "io_error",
            PreferencesError::Corrupt(_) => "preferences_corrupt",
            PreferencesError::NoConfigDir(_) => "preferences_unavailable",
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
fn preferences_path(app: &AppHandle) -> Result<PathBuf, AppErrorDto> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| PreferencesError::NoConfigDir(e.to_string()))?;
    Ok(dir.join(preferences::PREFERENCES_FILE))
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    base_dir: String,
    working_name: String,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    ProjectService::create_project(&state, &PathBuf::from(base_dir), &working_name)
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn open_project(
    state: State<'_, AppState>,
    package_path: String,
    force_stale_lock_recovery: bool,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    ProjectService::open_project(
        &state,
        &PathBuf::from(package_path),
        force_stale_lock_recovery,
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[tauri::command]
pub fn rename_project(
    state: State<'_, AppState>,
    project_id: String,
    new_name: String,
    expected_revision: i64,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    let id = parse_project_id(&project_id)?;
    ProjectService::rename_project(&state, id, &new_name, expected_revision)
        .map(Into::into)
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
    ProjectService::create_backup(&state, id, &PathBuf::from(backup_dir))
        .map(|p| p.display().to_string())
        .map_err(Into::into)
}

#[tauri::command]
pub fn restore_backup_as_copy(
    state: State<'_, AppState>,
    backup_path: String,
    destination_dir: String,
    new_working_name: Option<String>,
) -> Result<ProjectSummaryDto, AppErrorDto> {
    ProjectService::restore_backup_as_copy(
        &state,
        &PathBuf::from(backup_path),
        &PathBuf::from(destination_dir),
        new_working_name.as_deref(),
    )
    .map(Into::into)
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
pub fn get_preferences(app: AppHandle) -> Result<PreferencesDto, AppErrorDto> {
    let path = preferences_path(&app)?;
    Ok(preferences::load(&path)?.into())
}

#[tauri::command]
pub fn set_default_projects_dir(
    app: AppHandle,
    directory: Option<String>,
) -> Result<PreferencesDto, AppErrorDto> {
    let path = preferences_path(&app)?;
    let mut prefs = preferences::load(&path)?;
    prefs.default_projects_dir = directory.filter(|d| !d.is_empty()).map(PathBuf::from);
    preferences::save(&path, &prefs)?;
    Ok(prefs.into())
}

#[tauri::command]
pub fn set_default_backups_dir(
    app: AppHandle,
    directory: Option<String>,
) -> Result<PreferencesDto, AppErrorDto> {
    let path = preferences_path(&app)?;
    let mut prefs = preferences::load(&path)?;
    prefs.default_backups_dir = directory.filter(|d| !d.is_empty()).map(PathBuf::from);
    preferences::save(&path, &prefs)?;
    Ok(prefs.into())
}

/// Shows a native folder picker, optionally starting in `default_path`.
/// Returns `None` when the user cancels the dialog; this is never treated
/// as an error.
#[tauri::command]
pub fn pick_directory(app: AppHandle, default_path: Option<String>) -> Option<String> {
    let mut builder = app.dialog().file();
    if let Some(path) = default_path.filter(|p| !p.is_empty()) {
        builder = builder.set_directory(path);
    }
    builder
        .blocking_pick_folder()
        .and_then(|picked| picked.into_path().ok())
        .map(|p| p.display().to_string())
}
