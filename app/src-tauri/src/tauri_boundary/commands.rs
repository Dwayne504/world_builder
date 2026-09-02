//! Typed Tauri commands. Each command validates nothing itself beyond
//! parsing its DTO arguments -- all product/invariant validation happens in
//! `application`/`domain` -- and immediately delegates to
//! [`crate::application::ProjectService`].

use std::path::PathBuf;

use tauri::State;

use crate::application::{AppState, ProjectService};
use crate::domain::ProjectId;

use super::dto::{AppErrorDto, ProjectSummaryDto};

fn parse_project_id(raw: &str) -> Result<ProjectId, AppErrorDto> {
    ProjectId::parse(raw).map_err(|e| AppErrorDto {
        kind: "invalid_input".to_string(),
        message: e.to_string(),
    })
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
