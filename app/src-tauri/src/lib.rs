//! Worldcrafter application shell.
//!
//! Module boundaries and dependency direction (see
//! `docs/architecture/MILESTONE_01_ARCHITECTURE_PROPOSAL_V2.md` section 14):
//!
//! `tauri_boundary` -> `application` -> `domain` + `persistence` + `package` + `backup_recovery`
//!
//! `domain` depends on nothing else in this crate. `persistence`, `package`,
//! and `backup_recovery` never depend on `tauri`. Only `tauri_boundary`
//! depends on the `tauri` crate.

pub mod application;
pub mod backup_recovery;
pub mod domain;
pub mod package;
pub mod persistence;
pub mod tauri_boundary;

use application::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            tauri_boundary::create_project,
            tauri_boundary::open_project,
            tauri_boundary::rename_project,
            tauri_boundary::close_project,
            tauri_boundary::get_project_summary,
            tauri_boundary::create_backup,
            tauri_boundary::restore_backup_as_copy,
            tauri_boundary::list_categories,
            tauri_boundary::create_category,
            tauri_boundary::list_types,
            tauri_boundary::create_type,
            tauri_boundary::list_entries,
            tauri_boundary::create_entry,
            tauri_boundary::get_entry,
            tauri_boundary::update_entry_name,
            tauri_boundary::change_entry_structure,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
