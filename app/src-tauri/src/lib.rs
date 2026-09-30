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
pub mod application_home;
pub mod atomic_file;
pub mod backup_recovery;
pub mod domain;
pub mod package;
pub mod persistence;
pub mod preferences;
pub mod tauri_boundary;

use application::AppState;
use preferences::PreferencesStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let path = tauri_boundary::commands::preferences_path(&handle)
                .map_err(|e| -> Box<dyn std::error::Error> { e.message.into() })?;
            app.manage(application_home::RecentProjectsStore::new(
                path.with_file_name("recent-projects.sqlite"),
            ));
            app.manage(PreferencesStore::new(path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tauri_boundary::read_relationships,
            tauri_boundary::read_project_relationships,
            tauri_boundary::apply_relationships,
            tauri_boundary::preview_field_merge,
            tauri_boundary::merge_fields,
            tauri_boundary::read_field_catalog,
            tauri_boundary::apply_template_fields,
            tauri_boundary::delete_entry_field,
            tauri_boundary::automatic_backup_directory,
            tauri_boundary::read_fields,
            tauri_boundary::apply_fields,
            tauri_boundary::list_recent_projects,
            tauri_boundary::forget_recent_project,
            tauri_boundary::open_recent_project,
            tauri_boundary::create_project,
            tauri_boundary::open_project,
            tauri_boundary::rename_project,
            tauri_boundary::close_project,
            tauri_boundary::get_project_summary,
            tauri_boundary::list_open_projects,
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
            tauri_boundary::get_appearance,
            tauri_boundary::set_appearance,
            tauri_boundary::get_preferences,
            tauri_boundary::set_default_projects_dir,
            tauri_boundary::set_default_backups_dir,
            tauri_boundary::reset_preferences,
            tauri_boundary::preview_package_path,
            tauri_boundary::pick_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
