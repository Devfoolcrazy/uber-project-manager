pub use rusqlite;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

// group_concat separator for tag names: the ASCII unit separator can't
// appear in user input, unlike a comma.
const TAG_SEP: char = '\u{1f}';

// ---------- Types ----------

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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
pub struct ScanCandidate {
    pub name: String,
    pub path: String,
    pub has_git: bool,
    pub already_registered: bool,
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

// ---------- Database ----------

/// The app data directory the Tauri app resolves for this bundle identifier.
pub fn default_db_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME non défini".to_string())?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support/com.remy.uberprojectmanager"))
}

pub fn init_db(app_data_dir: PathBuf) -> Result<Connection, String> {
    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("projects.db");
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS projects (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            category    TEXT NOT NULL DEFAULT '',
            tags        TEXT NOT NULL DEFAULT '[]',
            path        TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS changelog (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            entry      TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tags (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );

        CREATE TABLE IF NOT EXISTS project_tags (
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (project_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS categories (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );",
    )
    .map_err(|e| e.to_string())?;

    for (column, ddl) in [
        (
            "repo",
            "ALTER TABLE projects ADD COLUMN repo TEXT NOT NULL DEFAULT ''",
        ),
        (
            "status",
            "ALTER TABLE projects ADD COLUMN status TEXT NOT NULL DEFAULT 'active'",
        ),
    ] {
        let present: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = ?1",
                [column],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if present == 0 {
            conn.execute(ddl, []).map_err(|e| e.to_string())?;
        }
    }

    migrate_json_tags(&conn)?;

    // Self-healing sync: any category name still referenced by a project is
    // guaranteed to exist in the referential (covers pre-normalization data).
    conn.execute(
        "INSERT OR IGNORE INTO categories (name)
         SELECT DISTINCT trim(category) FROM projects WHERE trim(category) != ''",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn)
}

// Databases created before tags were normalized stored them as a JSON array
// in projects.tags; import that content into tags/project_tags exactly once.
fn migrate_json_tags(conn: &Connection) -> Result<(), String> {
    let migrated: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM settings WHERE key = 'tags_migrated'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if migrated > 0 {
        return Ok(());
    }

    let rows: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, tags FROM projects")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        rows
    };

    for (project_id, tags_json) in rows {
        let names: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        for name in names {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [name])
                .map_err(|e| e.to_string())?;
            let tag_id: i64 = conn
                .query_row("SELECT id FROM tags WHERE name = ?1", [name], |row| {
                    row.get(0)
                })
                .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT OR IGNORE INTO project_tags (project_id, tag_id) VALUES (?1, ?2)",
                params![project_id, tag_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('tags_migrated', '1')",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Projects ----------

fn ensure_path_is_dir(path: &str) -> Result<(), String> {
    if Path::new(path).is_dir() {
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
        path_exists: Path::new(&path).exists(),
        path,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        changelog_count: row.get("changelog_count")?,
    })
}

const PROJECT_SELECT: &str = "SELECT p.*,
    (SELECT COUNT(*) FROM changelog c WHERE c.project_id = p.id) AS changelog_count,
    COALESCE((SELECT group_concat(name, char(31)) FROM (
        SELECT t.name FROM project_tags pt
        JOIN tags t ON t.id = pt.tag_id
        WHERE pt.project_id = p.id ORDER BY t.name
    )), '') AS tag_names
 FROM projects p";

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>, String> {
    let mut stmt = conn
        .prepare(&format!("{PROJECT_SELECT} ORDER BY p.updated_at DESC"))
        .map_err(|e| e.to_string())?;
    let projects = stmt
        .query_map([], row_to_project)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(projects)
}

/// Case-insensitive search across the project sheet and its changelog.
pub fn search_projects(conn: &Connection, query: &str) -> Result<Vec<Project>, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return list_projects(conn);
    }
    let matching_log_ids: std::collections::HashSet<i64> = {
        let mut stmt = conn
            .prepare("SELECT DISTINCT project_id FROM changelog WHERE lower(entry) LIKE ?1")
            .map_err(|e| e.to_string())?;
        let ids = stmt
            .query_map([format!("%{q}%")], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?;
        ids
    };
    Ok(list_projects(conn)?
        .into_iter()
        .filter(|p| {
            let haystack = format!(
                "{} {} {} {} {} {}",
                p.name,
                p.description,
                p.category,
                p.path,
                p.repo,
                p.tags.join(" ")
            )
            .to_lowercase();
            haystack.contains(&q) || matching_log_ids.contains(&p.id)
        })
        .collect())
}

/// Find one project by id or (case-insensitive) exact name.
pub fn find_project(conn: &Connection, ident: &str) -> Result<Project, String> {
    let projects = list_projects(conn)?;
    if let Ok(id) = ident.parse::<i64>() {
        if let Some(p) = projects.iter().find(|p| p.id == id) {
            return Ok(p.clone());
        }
    }
    let lower = ident.to_lowercase();
    let matches: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase() == lower)
        .collect();
    match matches.len() {
        1 => Ok(matches[0].clone()),
        0 => Err(format!("Aucun projet nommé « {ident} ».")),
        _ => Err(format!(
            "Plusieurs projets nommés « {ident} » — utilise l'id ({}).",
            matches
                .iter()
                .map(|p| p.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub fn create_project(conn: &Connection, input: &ProjectInput) -> Result<i64, String> {
    ensure_path_is_dir(&input.path)?;
    ensure_category_exists(conn, &input.category)?;
    conn.execute(
        "INSERT INTO projects (name, description, category, path, repo, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.name,
            input.description,
            input.category,
            input.path,
            input.repo,
            input.status
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    set_project_tags(conn, id, &input.tags)?;
    Ok(id)
}

pub fn update_project(conn: &Connection, id: i64, input: &ProjectInput) -> Result<(), String> {
    ensure_path_is_dir(&input.path)?;
    ensure_category_exists(conn, &input.category)?;
    conn.execute(
        "UPDATE projects SET name = ?1, description = ?2, category = ?3, path = ?4, repo = ?5,
         status = ?6, updated_at = datetime('now', 'localtime') WHERE id = ?7",
        params![
            input.name,
            input.description,
            input.category,
            input.path,
            input.repo,
            input.status,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    set_project_tags(conn, id, &input.tags)?;
    Ok(())
}

pub fn delete_project(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Tags ----------

pub fn list_tags(conn: &Connection) -> Result<Vec<Tag>, String> {
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

pub fn create_tag(conn: &Connection, name: &str) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom du tag est vide.".to_string());
    }
    let exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM tags WHERE name = ?1", [name], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("Le tag « {name} » existe déjà."));
    }
    conn.execute("INSERT INTO tags (name) VALUES (?1)", [name])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn rename_tag(conn: &Connection, id: i64, name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom du tag est vide.".to_string());
    }
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

pub fn delete_tag(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM tags WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Categories ----------

pub fn list_categories(conn: &Connection) -> Result<Vec<Category>, String> {
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

pub fn create_category(conn: &Connection, name: &str) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom de la catégorie est vide.".to_string());
    }
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("La catégorie « {name} » existe déjà."));
    }
    conn.execute("INSERT INTO categories (name) VALUES (?1)", [name])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn rename_category(conn: &Connection, id: i64, name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom de la catégorie est vide.".to_string());
    }
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

pub fn delete_category(conn: &Connection, id: i64) -> Result<(), String> {
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

// ---------- Changelog ----------

pub fn list_changelog(conn: &Connection, project_id: i64) -> Result<Vec<ChangelogEntry>, String> {
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

pub fn add_changelog_entry(
    conn: &Connection,
    project_id: i64,
    entry: &str,
) -> Result<i64, String> {
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

pub fn delete_changelog_entry(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM changelog WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Settings ----------

pub fn get_settings(
    conn: &Connection,
) -> Result<std::collections::HashMap<String, String>, String> {
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

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn get_setting_or(conn: &Connection, key: &str, default: &str) -> String {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| default.to_string())
}

// ---------- Scan ----------

pub fn scan_directory(conn: &Connection, root: &str) -> Result<Vec<ScanCandidate>, String> {
    let registered: Vec<String> = {
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
    let entries = std::fs::read_dir(root).map_err(|e| e.to_string())?;
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

// ---------- Insights ----------

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
    if !Path::new(path).join(".git").exists() {
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
    dir: &Path,
    root: &Path,
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

pub fn project_insights(path: &str) -> Result<ProjectInsights, String> {
    let root = Path::new(path);
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
        git: git_info(path),
        size_bytes,
        reclaim_bytes,
        artifacts,
    })
}

// ---------- Export ----------

pub fn export_data(conn: &Connection, dest: &str) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("{PROJECT_SELECT} ORDER BY p.name"))
        .map_err(|e| e.to_string())?;
    let projects = stmt
        .query_map([], row_to_project)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, entry, created_at FROM changelog
             ORDER BY project_id, created_at",
        )
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

    let categories: Vec<String> = list_categories(conn)?
        .into_iter()
        .map(|c| c.name)
        .collect();
    let tags: Vec<String> = list_tags(conn)?.into_iter().map(|t| t.name).collect();
    let settings = get_settings(conn)?;

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
    std::fs::write(dest, json).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Launchers ----------

pub fn open_in_finder(path: &str) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Opens either a URL (repo) or a filesystem path (backup dir): macOS `open`
// handles both.
pub fn open_repo(repo: &str) -> Result<(), String> {
    if repo.trim().is_empty() {
        return Err("Aucun repo ou dossier de sauvegarde renseigné.".to_string());
    }
    Command::new("open")
        .arg(repo.trim())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn open_in_ide(conn: &Connection, path: &str) -> Result<(), String> {
    let ide_command = get_setting_or(conn, "ide_command", "code");
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

pub fn open_in_terminal(conn: &Connection, path: &str) -> Result<(), String> {
    let terminal_app = get_setting_or(conn, "terminal_app", "Terminal");
    Command::new("open")
        .args(["-a", &terminal_app, path])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
