use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct Db(pub Mutex<Connection>);

// group_concat separator for tag names: the ASCII unit separator can't
// appear in user input, unlike a comma.
const TAG_SEP: char = '\u{1f}';

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub path: String,
    pub repo: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub changelog_count: i64,
    pub path_exists: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectInput {
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub path: String,
    pub repo: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub usage_count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub usage_count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChangelogEntry {
    pub id: i64,
    pub project_id: i64,
    pub entry: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitInfo {
    pub branch: String,
    pub last_commit_at: Option<String>,
    pub dirty: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArtifactDir {
    pub rel_path: String,
    pub bytes: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectInsights {
    pub techs: Vec<String>,
    pub git: Option<GitInfo>,
    pub size_bytes: u64,
    pub reclaim_bytes: u64,
    pub artifacts: Vec<ArtifactDir>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScanCandidate {
    pub name: String,
    pub path: String,
    pub has_git: bool,
    pub already_registered: bool,
}

fn ensure_path_is_dir(path: &str) -> Result<(), String> {
    if std::path::Path::new(path).is_dir() {
        Ok(())
    } else {
        Err(format!(
            "Le dossier « {path} » n'existe pas (supprimé, déplacé ou renommé ?)."
        ))
    }
}

fn ensure_category_exists(conn: &Connection, category: &str) -> Result<(), String> {
    if category.is_empty() {
        return Ok(());
    }
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE name = ?1",
            [category],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        return Err(format!(
            "La catégorie « {category} » n'existe pas. Crée-la d'abord dans l'administration."
        ));
    }
    Ok(())
}

fn set_project_tags(conn: &Connection, project_id: i64, tags: &[String]) -> Result<(), String> {
    conn.execute(
        "DELETE FROM project_tags WHERE project_id = ?1",
        params![project_id],
    )
    .map_err(|e| e.to_string())?;
    for name in tags {
        // Tags are admin-managed: only link names that already exist.
        let tag_id: Option<i64> = conn
            .query_row("SELECT id FROM tags WHERE name = ?1", [name], |row| {
                row.get(0)
            })
            .ok();
        if let Some(tag_id) = tag_id {
            conn.execute(
                "INSERT OR IGNORE INTO project_tags (project_id, tag_id) VALUES (?1, ?2)",
                params![project_id, tag_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    let tag_names: String = row.get("tag_names")?;
    let path: String = row.get("path")?;
    Ok(Project {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        category: row.get("category")?,
        tags: tag_names
            .split(TAG_SEP)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
        repo: row.get("repo")?,
        status: row.get("status")?,
        path_exists: std::path::Path::new(&path).exists(),
        path,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        changelog_count: row.get("changelog_count")?,
    })
}

#[tauri::command]
pub fn list_projects(db: State<Db>) -> Result<Vec<Project>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT p.*,
                (SELECT COUNT(*) FROM changelog c WHERE c.project_id = p.id) AS changelog_count,
                COALESCE((SELECT group_concat(name, char(31)) FROM (
                    SELECT t.name FROM project_tags pt
                    JOIN tags t ON t.id = pt.tag_id
                    WHERE pt.project_id = p.id ORDER BY t.name
                )), '') AS tag_names
             FROM projects p ORDER BY p.updated_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let projects = stmt
        .query_map([], row_to_project)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(projects)
}

#[tauri::command]
pub fn create_project(db: State<Db>, input: ProjectInput) -> Result<i64, String> {
    ensure_path_is_dir(&input.path)?;
    let conn = db.0.lock().unwrap();
    ensure_category_exists(&conn, &input.category)?;
    conn.execute(
        "INSERT INTO projects (name, description, category, path, repo, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![input.name, input.description, input.category, input.path, input.repo, input.status],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    set_project_tags(&conn, id, &input.tags)?;
    Ok(id)
}

#[tauri::command]
pub fn update_project(db: State<Db>, id: i64, input: ProjectInput) -> Result<(), String> {
    ensure_path_is_dir(&input.path)?;
    let conn = db.0.lock().unwrap();
    ensure_category_exists(&conn, &input.category)?;
    conn.execute(
        "UPDATE projects SET name = ?1, description = ?2, category = ?3, path = ?4, repo = ?5,
         status = ?6, updated_at = datetime('now', 'localtime') WHERE id = ?7",
        params![input.name, input.description, input.category, input.path, input.repo, input.status, id],
    )
    .map_err(|e| e.to_string())?;
    set_project_tags(&conn, id, &input.tags)?;
    Ok(())
}

#[tauri::command]
pub fn delete_project(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_tags(db: State<Db>) -> Result<Vec<Tag>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name,
                (SELECT COUNT(*) FROM project_tags pt WHERE pt.tag_id = t.id) AS usage_count
             FROM tags t ORDER BY t.name",
        )
        .map_err(|e| e.to_string())?;
    let tags = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                usage_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(tags)
}

#[tauri::command]
pub fn create_tag(db: State<Db>, name: String) -> Result<i64, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Le nom du tag est vide.".to_string());
    }
    let conn = db.0.lock().unwrap();
    let exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM tags WHERE name = ?1", [&name], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("Le tag « {name} » existe déjà."));
    }
    conn.execute("INSERT INTO tags (name) VALUES (?1)", [&name])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn rename_tag(db: State<Db>, id: i64, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Le nom du tag est vide.".to_string());
    }
    let conn = db.0.lock().unwrap();
    let clash: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tags WHERE name = ?1 AND id != ?2",
            params![name, id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if clash > 0 {
        return Err(format!("Le tag « {name} » existe déjà."));
    }
    conn.execute(
        "UPDATE tags SET name = ?1 WHERE id = ?2",
        params![name, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_tag(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM tags WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_categories(db: State<Db>) -> Result<Vec<Category>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name,
                (SELECT COUNT(*) FROM projects p WHERE p.category = c.name) AS usage_count
             FROM categories c ORDER BY c.name",
        )
        .map_err(|e| e.to_string())?;
    let categories = stmt
        .query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                usage_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(categories)
}

#[tauri::command]
pub fn create_category(db: State<Db>, name: String) -> Result<i64, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Le nom de la catégorie est vide.".to_string());
    }
    let conn = db.0.lock().unwrap();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE name = ?1",
            [&name],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("La catégorie « {name} » existe déjà."));
    }
    conn.execute("INSERT INTO categories (name) VALUES (?1)", [&name])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn rename_category(db: State<Db>, id: i64, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Le nom de la catégorie est vide.".to_string());
    }
    let conn = db.0.lock().unwrap();
    let old_name: String = conn
        .query_row("SELECT name FROM categories WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    let clash: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE name = ?1 AND id != ?2",
            params![name, id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if clash > 0 {
        return Err(format!("La catégorie « {name} » existe déjà."));
    }
    conn.execute(
        "UPDATE categories SET name = ?1 WHERE id = ?2",
        params![name, id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE projects SET category = ?1 WHERE category = ?2",
        params![name, old_name],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_category(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    let name: String = conn
        .query_row("SELECT name FROM categories WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    // Projects that used it fall back to "no category".
    conn.execute(
        "UPDATE projects SET category = '' WHERE category = ?1",
        [&name],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_changelog(db: State<Db>, project_id: i64) -> Result<Vec<ChangelogEntry>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT * FROM changelog WHERE project_id = ?1 ORDER BY created_at DESC, id DESC")
        .map_err(|e| e.to_string())?;
    let entries = stmt
        .query_map(params![project_id], |row| {
            Ok(ChangelogEntry {
                id: row.get("id")?,
                project_id: row.get("project_id")?,
                entry: row.get("entry")?,
                created_at: row.get("created_at")?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(entries)
}

#[tauri::command]
pub fn add_changelog_entry(db: State<Db>, project_id: i64, entry: String) -> Result<i64, String> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO changelog (project_id, entry) VALUES (?1, ?2)",
        params![project_id, entry],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE projects SET updated_at = datetime('now', 'localtime') WHERE id = ?1",
        params![project_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn delete_changelog_entry(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM changelog WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(db: State<Db>) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let map = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    Ok(map)
}

#[tauri::command]
pub fn set_setting(db: State<Db>, key: String, value: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn scan_directory(db: State<Db>, root: String) -> Result<Vec<ScanCandidate>, String> {
    let registered: Vec<String> = {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT path FROM projects")
            .map_err(|e| e.to_string())?;
        let paths = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(|e| e.to_string())?;
        paths
    };
    let normalize = |p: &str| p.trim_end_matches('/').to_string();
    let registered: std::collections::HashSet<String> =
        registered.iter().map(|p| normalize(p)).collect();

    let mut candidates = Vec::new();
    let entries = std::fs::read_dir(&root).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() || name.starts_with('.') {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        candidates.push(ScanCandidate {
            has_git: path.join(".git").exists(),
            already_registered: registered.contains(&normalize(&path_str)),
            name,
            path: path_str,
        });
    }
    candidates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(candidates)
}

#[tauri::command]
pub fn export_data(db: State<Db>, dest: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT p.*,
                (SELECT COUNT(*) FROM changelog c WHERE c.project_id = p.id) AS changelog_count,
                COALESCE((SELECT group_concat(name, char(31)) FROM (
                    SELECT t.name FROM project_tags pt
                    JOIN tags t ON t.id = pt.tag_id
                    WHERE pt.project_id = p.id ORDER BY t.name
                )), '') AS tag_names
             FROM projects p ORDER BY p.name",
        )
        .map_err(|e| e.to_string())?;
    let projects = stmt
        .query_map([], row_to_project)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, project_id, entry, created_at FROM changelog ORDER BY project_id, created_at")
        .map_err(|e| e.to_string())?;
    let changelog: Vec<ChangelogEntry> = stmt
        .query_map([], |row| {
            Ok(ChangelogEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                entry: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    let mut entries_by_project: std::collections::HashMap<i64, Vec<&ChangelogEntry>> =
        std::collections::HashMap::new();
    for entry in &changelog {
        entries_by_project
            .entry(entry.project_id)
            .or_default()
            .push(entry);
    }

    let categories: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM categories ORDER BY name")
            .map_err(|e| e.to_string())?;
        let names = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(|e| e.to_string())?;
        names
    };

    let tags: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM tags ORDER BY name")
            .map_err(|e| e.to_string())?;
        let names = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(|e| e.to_string())?;
        names
    };

    let settings: std::collections::HashMap<String, String> = {
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| e.to_string())?;
        let map = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?;
        map
    };

    let exported_at: String = conn
        .query_row("SELECT datetime('now', 'localtime')", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "exported_at": exported_at,
        "projects": projects.iter().map(|p| serde_json::json!({
            "name": p.name,
            "description": p.description,
            "category": p.category,
            "tags": p.tags,
            "path": p.path,
            "repo": p.repo,
            "status": p.status,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
            "changelog": entries_by_project.get(&p.id).map(|entries| {
                entries.iter().map(|e| serde_json::json!({
                    "entry": e.entry,
                    "created_at": e.created_at,
                })).collect::<Vec<_>>()
            }).unwrap_or_default(),
        })).collect::<Vec<_>>(),
        "categories": categories,
        "tags": tags,
        "settings": settings,
    });

    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&dest, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reveal_database(app: tauri::AppHandle) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Command::new("open")
        .arg(dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

const TECH_MARKERS: &[(&str, &str)] = &[
    ("Cargo.toml", "rust"),
    ("package.json", "node"),
    ("pyproject.toml", "python"),
    ("requirements.txt", "python"),
    ("setup.py", "python"),
    ("go.mod", "go"),
    ("pom.xml", "java"),
    ("build.gradle", "java"),
    ("Gemfile", "ruby"),
    ("composer.json", "php"),
    ("Package.swift", "swift"),
    ("Dockerfile", "docker"),
];

// Build artifacts that can be regenerated, hence counted as reclaimable.
const ARTIFACT_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".turbo",
];

fn git_run(path: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn git_info(path: &str) -> Option<GitInfo> {
    if !std::path::Path::new(path).join(".git").exists() {
        return None;
    }
    let branch = git_run(path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let last_commit_at =
        git_run(path, &["log", "-1", "--format=%cI"]).filter(|s| !s.is_empty());
    let dirty = git_run(path, &["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    Some(GitInfo {
        branch,
        last_commit_at,
        dirty,
    })
}

// Returns the subtree size, collecting the top-most artifact directories on
// the way. Symlinks are not followed.
fn walk_size(
    dir: &std::path::Path,
    root: &std::path::Path,
    in_artifact: bool,
    artifacts: &mut Vec<ArtifactDir>,
) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut size = 0u64;
    for entry in entries.flatten() {
        let Ok(meta) = entry.path().symlink_metadata() else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_artifact = !in_artifact && ARTIFACT_DIRS.contains(&name.as_str());
            let sub = walk_size(&entry.path(), root, in_artifact || is_artifact, artifacts);
            if is_artifact {
                artifacts.push(ArtifactDir {
                    rel_path: entry
                        .path()
                        .strip_prefix(root)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or(name),
                    bytes: sub,
                });
            }
            size += sub;
        } else {
            size += meta.len();
        }
    }
    size
}

#[tauri::command]
pub fn project_insights(path: String) -> Result<ProjectInsights, String> {
    let root = std::path::Path::new(&path);
    if !root.is_dir() {
        return Err(format!("Le dossier « {path} » n'existe pas."));
    }

    let mut techs = Vec::new();
    for (marker, tech) in TECH_MARKERS {
        if root.join(marker).exists() && !techs.contains(&tech.to_string()) {
            techs.push(tech.to_string());
        }
    }

    let mut artifacts = Vec::new();
    let size_bytes = walk_size(root, root, false, &mut artifacts);
    artifacts.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    let reclaim_bytes = artifacts.iter().map(|a| a.bytes).sum();

    Ok(ProjectInsights {
        techs,
        git: git_info(&path),
        size_bytes,
        reclaim_bytes,
        artifacts,
    })
}

#[tauri::command]
pub fn open_in_finder(path: String) -> Result<(), String> {
    Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Opens either a URL (repo) or a filesystem path (backup dir): macOS `open`
// handles both.
#[tauri::command]
pub fn open_repo(repo: String) -> Result<(), String> {
    if repo.trim().is_empty() {
        return Err("Aucun repo ou dossier de sauvegarde renseigné.".to_string());
    }
    Command::new("open")
        .arg(repo.trim())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_in_ide(db: State<Db>, path: String) -> Result<(), String> {
    let ide_command = {
        let conn = db.0.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'ide_command'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "code".to_string())
    };
    // The configured command may contain flags (e.g. "open -a Cursor"),
    // so run it through the shell with the path safely single-quoted.
    let quoted = format!("'{}'", path.replace('\'', r"'\''"));
    Command::new("/bin/sh")
        .arg("-lc")
        .arg(format!("{ide_command} {quoted}"))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_in_terminal(db: State<Db>, path: String) -> Result<(), String> {
    let terminal_app = {
        let conn = db.0.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'terminal_app'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "Terminal".to_string())
    };
    Command::new("open")
        .args(["-a", &terminal_app, &path])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
