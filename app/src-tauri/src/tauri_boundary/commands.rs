//! Typed Tauri commands. Each command validates nothing itself beyond
//! parsing its DTO arguments -- all product/invariant validation happens in
//! `application`/`domain` -- and immediately delegates to
//! [`crate::application::ProjectService`].

use std::path::PathBuf;

use tauri::State;

use crate::application::{AppState, ProjectService};
use crate::domain::{CategoryId, EntryId, ProjectId, TypeId};

use super::dto::{AppErrorDto, CategoryDto, EntryDto, ProjectSummaryDto, TypeDto};

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
