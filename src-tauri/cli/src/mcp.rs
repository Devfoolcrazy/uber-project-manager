//! Minimal MCP server (stdio transport, newline-delimited JSON-RPC 2.0).
//! Exposes the project registry as tools for Claude Code and other clients.

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use uberpm_core as core;
use uberpm_core::rusqlite::Connection;

pub fn serve(conn: Connection) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let method = msg["method"].as_str().unwrap_or_default().to_string();
        let id = msg["id"].clone();
        // Notifications (no id) never get a response.
        if id.is_null() {
            continue;
        }
        let response = match method.as_str() {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": msg["params"]["protocolVersion"]
                        .as_str()
                        .unwrap_or("2025-06-18"),
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "uberpm", "version": env!("CARGO_PKG_VERSION") }
                }
            }),
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tool_definitions() }
            }),
            "tools/call" => {
                let name = msg["params"]["name"].as_str().unwrap_or_default();
                let args = &msg["params"]["arguments"];
                match call_tool(&conn, name, args) {
                    Ok(text) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "content": [{ "type": "text", "text": text }] }
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": format!("Erreur : {e}") }],
                            "isError": true
                        }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Méthode inconnue : {method}") }
            }),
        };
        let _ = writeln!(stdout, "{response}");
        let _ = stdout.flush();
    }
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "list_projects",
            "description": "Liste les projets référencés. Par défaut les projets en cours (actifs + en pause) ; archived=true pour les terminés/abandonnés.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archived": { "type": "boolean", "description": "true pour lister la vue Archivés" }
                }
            }
        },
        {
            "name": "search_projects",
            "description": "Recherche plein texte (insensible à la casse) dans les noms, descriptions, catégories, tags, chemins, repos et changelogs.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }
        },
        {
            "name": "create_project",
            "description": "Référence un nouveau projet. Le chemin doit être un dossier existant. Par défaut, crée la catégorie/les tags manquants.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Chemin absolu du dossier" },
                    "name": { "type": "string", "description": "Défaut : nom du dossier" },
                    "description": { "type": "string" },
                    "category": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "repo": { "type": "string", "description": "URL de repo ou chemin de sauvegarde" },
                    "status": { "type": "string", "enum": ["active", "paused", "done", "dropped"] },
                    "create_missing_refs": { "type": "boolean", "description": "Créer catégorie/tags manquants (défaut true)" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "update_project",
            "description": "Met à jour la fiche d'un projet (identifié par nom ou id). Seuls les champs fournis changent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "Nom exact ou id" },
                    "name": { "type": "string" },
                    "description": { "type": "string" },
                    "category": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "path": { "type": "string" },
                    "repo": { "type": "string" },
                    "status": { "type": "string", "enum": ["active", "paused", "done", "dropped"] }
                },
                "required": ["project"]
            }
        },
        {
            "name": "scan_directory",
            "description": "Liste les sous-dossiers non référencés d'un dossier racine. add_missing=true pour les référencer tous.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "root": { "type": "string" },
                    "add_missing": { "type": "boolean" }
                },
                "required": ["root"]
            }
        },
        {
            "name": "add_changelog_entry",
            "description": "Ajoute une note datée au changelog d'un projet (nom ou id).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "entry": { "type": "string" }
                },
                "required": ["project", "entry"]
            }
        },
        {
            "name": "get_changelog",
            "description": "Retourne le changelog d'un projet (nom ou id).",
            "inputSchema": {
                "type": "object",
                "properties": { "project": { "type": "string" } },
                "required": ["project"]
            }
        },
        {
            "name": "project_insights",
            "description": "Analyse le dossier d'un projet : branche git, dernier commit, modifs non commitées, technos détectées, taille disque et dossiers récupérables.",
            "inputSchema": {
                "type": "object",
                "properties": { "project": { "type": "string" } },
                "required": ["project"]
            }
        },
        {
            "name": "list_categories",
            "description": "Liste les catégories du référentiel avec leur nombre de projets.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "list_tags",
            "description": "Liste les tags du référentiel avec leur nombre de projets.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

fn str_arg(args: &Value, key: &str) -> Option<String> {
    args[key].as_str().map(String::from)
}

fn tags_arg(args: &Value) -> Option<Vec<String>> {
    args["tags"].as_array().map(|a| {
        a.iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect()
    })
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

fn call_tool(conn: &Connection, name: &str, args: &Value) -> Result<String, String> {
    match name {
        "list_projects" => {
            let archived = args["archived"].as_bool().unwrap_or(false);
            let projects: Vec<_> = core::list_projects(conn)?
                .into_iter()
                .filter(|p| (p.status == "done" || p.status == "dropped") == archived)
                .collect();
            to_json(&projects)
        }
        "search_projects" => {
            let query = str_arg(args, "query").ok_or("query manquant")?;
            to_json(&core::search_projects(conn, &query)?)
        }
        "create_project" => {
            let path = str_arg(args, "path").ok_or("path manquant")?;
            let category = str_arg(args, "category").unwrap_or_default();
            let tags = tags_arg(args).unwrap_or_default();
            if args["create_missing_refs"].as_bool().unwrap_or(true) {
                crate::ensure_refs(conn, &category, &tags)?;
            }
            let name = str_arg(args, "name").unwrap_or_else(|| {
                std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            });
            let id = core::create_project(
                conn,
                &core::ProjectInput {
                    name: name.clone(),
                    description: str_arg(args, "description").unwrap_or_default(),
                    category,
                    tags,
                    path,
                    repo: str_arg(args, "repo").unwrap_or_default(),
                    status: str_arg(args, "status").unwrap_or_else(|| "active".to_string()),
                },
            )?;
            Ok(format!("Projet « {name} » référencé (id {id})."))
        }
        "update_project" => {
            let ident = str_arg(args, "project").ok_or("project manquant")?;
            let p = core::find_project(conn, &ident)?;
            let category = str_arg(args, "category").unwrap_or(p.category);
            let tags = tags_arg(args).unwrap_or(p.tags);
            crate::ensure_refs(conn, &category, &tags)?;
            let input = core::ProjectInput {
                name: str_arg(args, "name").unwrap_or(p.name),
                description: str_arg(args, "description").unwrap_or(p.description),
                category,
                tags,
                path: str_arg(args, "path").unwrap_or(p.path),
                repo: str_arg(args, "repo").unwrap_or(p.repo),
                status: str_arg(args, "status").unwrap_or(p.status),
            };
            core::update_project(conn, p.id, &input)?;
            Ok(format!("Projet « {} » mis à jour.", input.name))
        }
        "scan_directory" => {
            let root = str_arg(args, "root").ok_or("root manquant")?;
            let fresh: Vec<_> = core::scan_directory(conn, &root)?
                .into_iter()
                .filter(|c| !c.already_registered)
                .collect();
            if args["add_missing"].as_bool().unwrap_or(false) {
                for c in &fresh {
                    core::create_project(
                        conn,
                        &core::ProjectInput {
                            name: c.name.clone(),
                            path: c.path.clone(),
                            status: "active".to_string(),
                            ..Default::default()
                        },
                    )?;
                }
                Ok(format!(
                    "{} projet(s) référencé(s) : {}",
                    fresh.len(),
                    fresh
                        .iter()
                        .map(|c| c.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            } else {
                to_json(&fresh)
            }
        }
        "add_changelog_entry" => {
            let ident = str_arg(args, "project").ok_or("project manquant")?;
            let entry = str_arg(args, "entry").ok_or("entry manquant")?;
            let p = core::find_project(conn, &ident)?;
            core::add_changelog_entry(conn, p.id, &entry)?;
            Ok(format!("Note ajoutée au changelog de « {} ».", p.name))
        }
        "get_changelog" => {
            let ident = str_arg(args, "project").ok_or("project manquant")?;
            let p = core::find_project(conn, &ident)?;
            to_json(&core::list_changelog(conn, p.id)?)
        }
        "project_insights" => {
            let ident = str_arg(args, "project").ok_or("project manquant")?;
            let p = core::find_project(conn, &ident)?;
            to_json(&core::project_insights(&p.path)?)
        }
        "list_categories" => to_json(&core::list_categories(conn)?),
        "list_tags" => to_json(&core::list_tags(conn)?),
        _ => Err(format!("Outil inconnu : {name}")),
    }
}
