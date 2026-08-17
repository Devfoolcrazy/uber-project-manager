mod commands;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let conn = uberpm_core::init_db(app_data_dir).map_err(std::io::Error::other)?;
            app.manage(commands::Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::list_tags,
            commands::create_tag,
            commands::rename_tag,
            commands::delete_tag,
            commands::list_categories,
            commands::create_category,
            commands::rename_category,
            commands::delete_category,
            commands::open_repo,
            commands::project_insights,
            commands::export_data,
            commands::reveal_database,
            commands::list_changelog,
            commands::add_changelog_entry,
            commands::delete_changelog_entry,
            commands::get_settings,
            commands::set_setting,
            commands::scan_directory,
            commands::open_in_finder,
            commands::open_in_ide,
            commands::open_in_terminal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
