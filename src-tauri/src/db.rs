use rusqlite::{params, Connection};
use std::path::PathBuf;

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

    let has_repo: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'repo'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if has_repo == 0 {
        conn.execute(
            "ALTER TABLE projects ADD COLUMN repo TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    let has_status: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'status'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if has_status == 0 {
        conn.execute(
            "ALTER TABLE projects ADD COLUMN status TEXT NOT NULL DEFAULT 'active'",
            [],
        )
        .map_err(|e| e.to_string())?;
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
