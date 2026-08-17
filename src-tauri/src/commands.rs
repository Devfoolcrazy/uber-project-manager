use std::sync::Mutex;
use uberpm_core::rusqlite::Connection;
use tauri::{Manager, State};
use uberpm_core as core;
use uberpm_core::{
    Category, ChangelogEntry, Project, ProjectInput, ProjectInsights, ScanCandidate, Tag,
};

pub struct Db(pub Mutex<Connection>);

#[tauri::command]
pub fn list_projects(db: State<Db>) -> Result<Vec<Project>, String> {
    core::list_projects(&db.0.lock().unwrap())
}

#[tauri::command]
pub fn create_project(db: State<Db>, input: ProjectInput) -> Result<i64, String> {
    core::create_project(&db.0.lock().unwrap(), &input)
}

#[tauri::command]
pub fn update_project(db: State<Db>, id: i64, input: ProjectInput) -> Result<(), String> {
    core::update_project(&db.0.lock().unwrap(), id, &input)
}

#[tauri::command]
pub fn delete_project(db: State<Db>, id: i64) -> Result<(), String> {
    core::delete_project(&db.0.lock().unwrap(), id)
}

#[tauri::command]
pub fn list_tags(db: State<Db>) -> Result<Vec<Tag>, String> {
    core::list_tags(&db.0.lock().unwrap())
}

#[tauri::command]
pub fn create_tag(db: State<Db>, name: String) -> Result<i64, String> {
    core::create_tag(&db.0.lock().unwrap(), &name)
}

#[tauri::command]
pub fn rename_tag(db: State<Db>, id: i64, name: String) -> Result<(), String> {
    core::rename_tag(&db.0.lock().unwrap(), id, &name)
}

#[tauri::command]
pub fn delete_tag(db: State<Db>, id: i64) -> Result<(), String> {
    core::delete_tag(&db.0.lock().unwrap(), id)
}

#[tauri::command]
pub fn list_categories(db: State<Db>) -> Result<Vec<Category>, String> {
    core::list_categories(&db.0.lock().unwrap())
}

#[tauri::command]
pub fn create_category(db: State<Db>, name: String) -> Result<i64, String> {
    core::create_category(&db.0.lock().unwrap(), &name)
}

#[tauri::command]
pub fn rename_category(db: State<Db>, id: i64, name: String) -> Result<(), String> {
    core::rename_category(&db.0.lock().unwrap(), id, &name)
}

#[tauri::command]
pub fn delete_category(db: State<Db>, id: i64) -> Result<(), String> {
    core::delete_category(&db.0.lock().unwrap(), id)
}

#[tauri::command]
pub fn list_changelog(db: State<Db>, project_id: i64) -> Result<Vec<ChangelogEntry>, String> {
    core::list_changelog(&db.0.lock().unwrap(), project_id)
}

#[tauri::command]
pub fn add_changelog_entry(db: State<Db>, project_id: i64, entry: String) -> Result<i64, String> {
    core::add_changelog_entry(&db.0.lock().unwrap(), project_id, &entry)
}

#[tauri::command]
pub fn delete_changelog_entry(db: State<Db>, id: i64) -> Result<(), String> {
    core::delete_changelog_entry(&db.0.lock().unwrap(), id)
}

#[tauri::command]
pub fn get_settings(db: State<Db>) -> Result<std::collections::HashMap<String, String>, String> {
    core::get_settings(&db.0.lock().unwrap())
}

#[tauri::command]
pub fn set_setting(db: State<Db>, key: String, value: String) -> Result<(), String> {
    core::set_setting(&db.0.lock().unwrap(), &key, &value)
}

#[tauri::command]
pub fn scan_directory(db: State<Db>, root: String) -> Result<Vec<ScanCandidate>, String> {
    core::scan_directory(&db.0.lock().unwrap(), &root)
}

#[tauri::command]
pub fn project_insights(path: String) -> Result<ProjectInsights, String> {
    core::project_insights(&path)
}

#[tauri::command]
pub fn export_data(db: State<Db>, dest: String) -> Result<(), String> {
    core::export_data(&db.0.lock().unwrap(), &dest)
}

#[tauri::command]
pub fn reveal_database(app: tauri::AppHandle) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    core::open_in_finder(&dir.to_string_lossy())
}

#[tauri::command]
pub fn open_in_finder(path: String) -> Result<(), String> {
    core::open_in_finder(&path)
}

#[tauri::command]
pub fn open_repo(repo: String) -> Result<(), String> {
    core::open_repo(&repo)
}

#[tauri::command]
pub fn open_in_ide(db: State<Db>, path: String) -> Result<(), String> {
    core::open_in_ide(&db.0.lock().unwrap(), &path)
}

#[tauri::command]
pub fn open_in_terminal(db: State<Db>, path: String) -> Result<(), String> {
    core::open_in_terminal(&db.0.lock().unwrap(), &path)
}
