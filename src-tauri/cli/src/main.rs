mod mcp;

use clap::{Parser, Subcommand};
use uberpm_core as core;
use uberpm_core::rusqlite::Connection;

#[derive(Parser)]
#[command(
    name = "uberpm",
    about = "CLI et serveur MCP pour Uber Project Manager",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Liste les projets
    List {
        /// Sortie JSON
        #[arg(long)]
        json: bool,
        /// N'affiche que les projets archivés (terminé/abandonné)
        #[arg(long)]
        archived: bool,
        /// Filtre par catégorie
        #[arg(long)]
        category: Option<String>,
        /// Filtre par tag (répétable)
        #[arg(long)]
        tag: Vec<String>,
    },
    /// Recherche dans les fiches et changelogs
    Search {
        query: String,
        #[arg(long)]
        json: bool,
    },
    /// Référence un nouveau projet
    Add {
        /// Chemin du dossier (obligatoire, doit exister)
        #[arg(long)]
        path: String,
        /// Nom (défaut : nom du dossier)
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long, default_value = "")]
        category: String,
        /// Tags séparés par des virgules
        #[arg(long, default_value = "")]
        tags: String,
        #[arg(long, default_value = "")]
        repo: String,
        /// active | paused | done | dropped
        #[arg(long, default_value = "active")]
        status: String,
        /// Crée la catégorie et les tags manquants au lieu d'échouer
        #[arg(long)]
        create_missing: bool,
    },
    /// Scanne des dossiers racines et liste (ou ajoute) les projets manquants
    Scan {
        roots: Vec<String>,
        /// Référence automatiquement tous les dossiers non référencés
        #[arg(long)]
        add_missing: bool,
        #[arg(long)]
        json: bool,
    },
    /// Affiche ou complète le changelog d'un projet (nom ou id)
    Log {
        project: String,
        /// Ajoute une note au lieu de lister
        #[arg(long)]
        add: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Liste les catégories
    Categories {
        #[arg(long)]
        json: bool,
    },
    /// Liste les tags
    Tags {
        #[arg(long)]
        json: bool,
    },
    /// Ouvre un projet (nom ou id) dans l'IDE configuré
    Open {
        project: String,
        /// Ouvre dans le Finder plutôt que l'IDE
        #[arg(long)]
        finder: bool,
        /// Ouvre un terminal plutôt que l'IDE
        #[arg(long)]
        terminal: bool,
    },
    /// Démarre le serveur MCP (stdio) pour Claude Code & co
    Mcp,
}

fn open_db() -> Result<Connection, String> {
    core::init_db(core::default_db_dir()?)
}

fn is_archived(status: &str) -> bool {
    status == "done" || status == "dropped"
}

fn print_projects(projects: &[core::Project], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(projects).unwrap());
        return;
    }
    if projects.is_empty() {
        println!("Aucun projet.");
        return;
    }
    for p in projects {
        let tags = if p.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", p.tags.join(", "))
        };
        let missing = if p.path_exists { "" } else { "  ⚠ introuvable" };
        println!(
            "#{:<4} {:<30} {:<12} {}{}{}",
            p.id, p.name, p.category, p.path, tags, missing
        );
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::List {
            json,
            archived,
            category,
            tag,
        } => {
            let conn = open_db()?;
            let projects: Vec<_> = core::list_projects(&conn)?
                .into_iter()
                .filter(|p| is_archived(&p.status) == archived)
                .filter(|p| category.as_deref().is_none_or(|c| p.category == c))
                .filter(|p| tag.iter().all(|t| p.tags.contains(t)))
                .collect();
            print_projects(&projects, json);
        }
        Cmd::Search { query, json } => {
            let conn = open_db()?;
            let projects = core::search_projects(&conn, &query)?;
            print_projects(&projects, json);
        }
        Cmd::Add {
            path,
            name,
            description,
            category,
            tags,
            repo,
            status,
            create_missing,
        } => {
            let conn = open_db()?;
            let tags: Vec<String> = tags
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            if create_missing {
                ensure_refs(&conn, &category, &tags)?;
            }
            let name = name.unwrap_or_else(|| {
                std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            });
            let id = core::create_project(
                &conn,
                &core::ProjectInput {
                    name: name.clone(),
                    description,
                    category,
                    tags,
                    path,
                    repo,
                    status,
                },
            )?;
            println!("Projet « {name} » référencé (#{id}).");
        }
        Cmd::Scan {
            roots,
            add_missing,
            json,
        } => {
            if roots.is_empty() {
                return Err("Indique au moins un dossier racine.".to_string());
            }
            let conn = open_db()?;
            let mut fresh = Vec::new();
            for root in &roots {
                for c in core::scan_directory(&conn, root)? {
                    if !c.already_registered {
                        fresh.push(c);
                    }
                }
            }
            if add_missing {
                let mut added = 0;
                for c in &fresh {
                    core::create_project(
                        &conn,
                        &core::ProjectInput {
                            name: c.name.clone(),
                            path: c.path.clone(),
                            status: "active".to_string(),
                            ..Default::default()
                        },
                    )?;
                    added += 1;
                    println!("+ {} ({})", c.name, c.path);
                }
                println!("{added} projet(s) référencé(s).");
            } else if json {
                println!("{}", serde_json::to_string_pretty(&fresh).unwrap());
            } else if fresh.is_empty() {
                println!("Rien de nouveau : tout est déjà référencé.");
            } else {
                for c in &fresh {
                    let git = if c.has_git { " (git)" } else { "" };
                    println!("{}{}  {}", c.name, git, c.path);
                }
                println!(
                    "{} dossier(s) non référencé(s). Relance avec --add-missing pour les ajouter.",
                    fresh.len()
                );
            }
        }
        Cmd::Log { project, add, json } => {
            let conn = open_db()?;
            let p = core::find_project(&conn, &project)?;
            if let Some(entry) = add {
                core::add_changelog_entry(&conn, p.id, &entry)?;
                println!("Note ajoutée à « {} ».", p.name);
            } else {
                let entries = core::list_changelog(&conn, p.id)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&entries).unwrap());
                } else if entries.is_empty() {
                    println!("Aucune note pour « {} ».", p.name);
                } else {
                    for e in entries {
                        println!("[{}] {}", e.created_at, e.entry);
                    }
                }
            }
        }
        Cmd::Categories { json } => {
            let conn = open_db()?;
            let categories = core::list_categories(&conn)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&categories).unwrap());
            } else {
                for c in categories {
                    println!("{:<20} {} projet(s)", c.name, c.usage_count);
                }
            }
        }
        Cmd::Tags { json } => {
            let conn = open_db()?;
            let tags = core::list_tags(&conn)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tags).unwrap());
            } else {
                for t in tags {
                    println!("{:<20} {} projet(s)", t.name, t.usage_count);
                }
            }
        }
        Cmd::Open {
            project,
            finder,
            terminal,
        } => {
            let conn = open_db()?;
            let p = core::find_project(&conn, &project)?;
            if !p.path_exists {
                return Err(format!("Le dossier de « {} » est introuvable.", p.name));
            }
            if finder {
                core::open_in_finder(&p.path)?;
            } else if terminal {
                core::open_in_terminal(&conn, &p.path)?;
            } else {
                core::open_in_ide(&conn, &p.path)?;
            }
        }
        Cmd::Mcp => {
            let conn = open_db()?;
            mcp::serve(conn);
        }
    }
    Ok(())
}

pub(crate) fn ensure_refs(
    conn: &Connection,
    category: &str,
    tags: &[String],
) -> Result<(), String> {
    if !category.is_empty()
        && !core::list_categories(conn)?
            .iter()
            .any(|c| c.name.eq_ignore_ascii_case(category))
    {
        core::create_category(conn, category)?;
    }
    for tag in tags {
        if !core::list_tags(conn)?
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case(tag))
        {
            core::create_tag(conn, tag)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
