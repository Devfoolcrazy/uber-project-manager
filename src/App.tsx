import { useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  Category,
  Project,
  ProjectInput,
  ProjectInsights,
  Settings,
  Tag,
} from "./types";
import {
  createProject,
  getSettings,
  listCategories,
  listProjects,
  listTags,
  openInIde,
  projectInsights,
  setSetting,
  updateProject,
} from "./api";
import { isArchived } from "./utils";
import AdminModal from "./components/AdminModal";
import Sidebar from "./components/Sidebar";
import ProjectRow from "./components/ProjectRow";
import ProjectDrawer from "./components/ProjectDrawer";
import ProjectForm from "./components/ProjectForm";
import ScanModal from "./components/ScanModal";
import SettingsModal from "./components/SettingsModal";
import "./App.css";

type SortOrder = "updated" | "created" | "name";

type Modal =
  | { kind: "none" }
  | { kind: "form"; project: Project | null }
  | { kind: "scan" }
  | { kind: "settings" };

export default function App() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [allTags, setAllTags] = useState<Tag[]>([]);
  const [allCategories, setAllCategories] = useState<Category[]>([]);
  const [settings, setSettings] = useState<Settings>({});
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<SortOrder>("updated");
  const [category, setCategory] = useState<string | null>(null);
  const [tagFilters, setTagFilters] = useState<string[]>([]);
  const [missingOnly, setMissingOnly] = useState(false);
  const [missingDismissed, setMissingDismissed] = useState(false);
  const [showArchived, setShowArchived] = useState(false);
  const [cursor, setCursor] = useState(0);
  const searchRef = useRef<HTMLInputElement>(null);
  // Per-path analysis cache: undefined = not fetched, null = fetch failed.
  const [insights, setInsights] = useState<
    Record<string, ProjectInsights | null>
  >({});
  const pendingInsights = useRef<Set<string>>(new Set());
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [modal, setModal] = useState<Modal>({ kind: "none" });
  const [tagsOpen, setTagsOpen] = useState(false);
  const [error, setError] = useState("");

  const refresh = () =>
    Promise.all([listProjects(), listTags(), listCategories()])
      .then(([p, t, c]) => {
        setProjects(p);
        setAllTags(t);
        setAllCategories(c);
      })
      .catch((e) => setError(String(e)));

  // The DB can change from outside the app (uberpm CLI, MCP server):
  // re-read it whenever the window regains focus.
  useEffect(() => {
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, []);

  useEffect(() => {
    refresh();
    getSettings()
      .then((s) => {
        setSettings(s);
        if (["updated", "created", "name"].includes(s.sort_order)) {
          setSort(s.sort_order as SortOrder);
        }
      })
      .catch((e) => setError(String(e)));
  }, []);

  const changeSort = (v: SortOrder) => {
    setSort(v);
    setSetting("sort_order", v).catch(() => {});
  };

  useEffect(() => {
    if (!error) return;
    const t = setTimeout(() => setError(""), 6000);
    return () => clearTimeout(t);
  }, [error]);

  // Fetch disk/git insights lazily, three projects at a time, so a large
  // list never blocks the UI.
  useEffect(() => {
    const missing = projects.filter(
      (p) =>
        p.path_exists &&
        !(p.path in insights) &&
        !pendingInsights.current.has(p.path),
    );
    if (missing.length === 0) return;
    missing.forEach((p) => pendingInsights.current.add(p.path));
    const queue = missing.map((p) => p.path);
    const worker = async () => {
      for (let path = queue.shift(); path; path = queue.shift()) {
        const current = path;
        try {
          const result = await projectInsights(current);
          setInsights((prev) => ({ ...prev, [current]: result }));
        } catch {
          setInsights((prev) => ({ ...prev, [current]: null }));
        } finally {
          pendingInsights.current.delete(current);
        }
      }
    };
    Promise.all([worker(), worker(), worker()]);
  }, [projects, insights]);

  const refreshInsight = (path: string) =>
    setInsights((prev) => {
      const next = { ...prev };
      delete next[path];
      return next;
    });

  const viewProjects = useMemo(
    () => projects.filter((p) => isArchived(p.status) === showArchived),
    [projects, showArchived],
  );
  const archivedCount = useMemo(
    () => projects.filter((p) => isArchived(p.status)).length,
    [projects],
  );

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = viewProjects.filter((p) => {
      if (category !== null) {
        const cat = p.category || "Sans catégorie";
        if (cat !== category) return false;
      }
      if (tagFilters.length > 0 && !tagFilters.every((t) => p.tags.includes(t)))
        return false;
      if (missingOnly && p.path_exists) return false;
      if (!q) return true;
      const haystack = [p.name, p.description, p.category, p.path, ...p.tags]
        .join(" ")
        .toLowerCase();
      return haystack.includes(q);
    });
    if (sort === "name") {
      list.sort((a, b) => a.name.localeCompare(b.name, "fr"));
    } else if (sort === "created") {
      list.sort((a, b) => b.created_at.localeCompare(a.created_at));
    } else {
      list.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
    }
    return list;
  }, [viewProjects, search, category, tagFilters, missingOnly, sort]);

  const missingCount = useMemo(
    () => viewProjects.filter((p) => !p.path_exists).length,
    [viewProjects],
  );

  const selected = projects.find((p) => p.id === selectedId) ?? null;

  // Keep the keyboard cursor in range when filters change the list.
  useEffect(() => {
    setCursor(0);
  }, [search, category, tagFilters, missingOnly, showArchived]);

  useEffect(() => {
    document
      .getElementById(`project-${filtered[cursor]?.id}`)
      ?.scrollIntoView({ block: "nearest" });
  }, [cursor, filtered]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
        return;
      }
      if (modal.kind !== "none" || tagsOpen) return;
      const el = document.activeElement;
      if (el !== searchRef.current && el !== document.body && el !== null)
        return;
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const delta = e.key === "ArrowDown" ? 1 : -1;
        setCursor((c) =>
          filtered.length === 0
            ? 0
            : Math.min(Math.max(c + delta, 0), filtered.length - 1),
        );
      } else if (e.key === "Enter") {
        const p = filtered[cursor];
        if (p?.path_exists) {
          openInIde(p.path).catch((err) => setError(String(err)));
        }
      } else if (e.key === "Escape") {
        if (search) setSearch("");
        else setSelectedId(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [modal, tagsOpen, filtered, cursor, search]);

  const relocate = async (p: Project) => {
    const dir = await open({ directory: true });
    if (typeof dir !== "string") return;
    try {
      await updateProject(p.id, {
        name: p.name,
        description: p.description,
        category: p.category,
        tags: p.tags,
        path: dir,
        repo: p.repo,
        status: p.status,
      });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const submitForm = async (input: ProjectInput) => {
    if (modal.kind !== "form") return;
    if (modal.project) {
      await updateProject(modal.project.id, input);
    } else {
      const id = await createProject(input);
      setSelectedId(id);
    }
    setModal({ kind: "none" });
    await refresh();
  };

  const toggleTag = (t: string) =>
    setTagFilters((prev) =>
      prev.includes(t) ? prev.filter((x) => x !== t) : [...prev, t],
    );

  return (
    <div className="app">
      <header className="topbar">
        <h1 className="app-title">
          Uber<span className="app-title-accent">·</span>PM
        </h1>
        <input
          ref={searchRef}
          autoFocus
          className="search"
          type="search"
          placeholder="Rechercher… (⌘K · ↑↓ naviguer · ⏎ ouvrir dans l'IDE)"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <select
          className="sort-select"
          title="Ordre de tri"
          value={sort}
          onChange={(e) => changeSort(e.target.value as SortOrder)}
        >
          <option value="updated">Dernière modif</option>
          <option value="created">Création</option>
          <option value="name">Nom</option>
        </select>
        <button
          className="secondary-btn"
          onClick={() => setModal({ kind: "scan" })}
        >
          Scanner
        </button>
        <button
          className="primary-btn"
          onClick={() => setModal({ kind: "form", project: null })}
        >
          Nouveau projet
        </button>
        <button className="ghost-btn" onClick={() => setTagsOpen(true)}>
          Admin
        </button>
        <button
          className="ghost-btn settings-btn"
          title="Réglages"
          onClick={() => setModal({ kind: "settings" })}
        >
          ⚙
        </button>
      </header>

      {missingCount > 0 && !missingDismissed && (
        <div className="banner-warning">
          <span>
            {missingCount} projet{missingCount > 1 ? "s" : ""} dont le dossier
            est introuvable (supprimé, déplacé ou renommé). Corrige le chemin
            via « Modifier », ou supprime la fiche.
          </span>
          <button
            className="secondary-btn"
            onClick={() => setMissingOnly(!missingOnly)}
          >
            {missingOnly ? "Tout afficher" : "Voir ces projets"}
          </button>
          <button
            className="ghost-btn"
            title="Masquer"
            onClick={() => {
              setMissingDismissed(true);
              setMissingOnly(false);
            }}
          >
            ✕
          </button>
        </div>
      )}

      {error && <div className="toast-error">{error}</div>}

      <div className="layout">
        <Sidebar
          projects={viewProjects}
          activeCount={projects.length - archivedCount}
          archivedCount={archivedCount}
          showArchived={showArchived}
          onShowArchived={setShowArchived}
          category={category}
          onCategory={setCategory}
          tags={tagFilters}
          onToggleTag={toggleTag}
        />

        <main className="content">
          {projects.length === 0 ? (
            <div className="empty-state">
              <h2>Aucun projet référencé</h2>
              <p>
                Ajoute ton premier projet, ou scanne un dossier racine pour
                importer tout ce qui s'y trouve.
              </p>
              <div className="empty-actions">
                <button
                  className="primary-btn"
                  onClick={() => setModal({ kind: "form", project: null })}
                >
                  Nouveau projet
                </button>
                <button
                  className="secondary-btn"
                  onClick={() => setModal({ kind: "scan" })}
                >
                  Scanner un dossier
                </button>
              </div>
            </div>
          ) : filtered.length === 0 ? (
            <div className="empty-state">
              <h2>Aucun résultat</h2>
              <p>Aucun projet ne correspond aux filtres actuels.</p>
              <div className="empty-actions">
                <button
                  className="secondary-btn"
                  onClick={() => {
                    setSearch("");
                    setCategory(null);
                    setTagFilters([]);
                    setMissingOnly(false);
                  }}
                >
                  Réinitialiser les filtres
                </button>
              </div>
            </div>
          ) : (
            <div className="project-list">
              {filtered.map((p, i) => (
                <ProjectRow
                  key={p.id}
                  project={p}
                  insight={insights[p.path]}
                  selected={p.id === selectedId}
                  cursor={i === cursor}
                  onSelect={() =>
                    setSelectedId(selectedId === p.id ? null : p.id)
                  }
                  onRelocate={() => relocate(p)}
                  onError={setError}
                />
              ))}
            </div>
          )}
        </main>

        {selected && (
          <ProjectDrawer
            project={selected}
            insight={insights[selected.path]}
            onRefreshInsight={() => refreshInsight(selected.path)}
            onClose={() => setSelectedId(null)}
            onEdit={() => setModal({ kind: "form", project: selected })}
            onRelocate={() => relocate(selected)}
            onChanged={refresh}
            onDeleted={() => {
              setSelectedId(null);
              refresh();
            }}
            onError={setError}
          />
        )}
      </div>

      {modal.kind === "form" && (
        <ProjectForm
          initial={modal.project}
          availableCategories={allCategories}
          availableTags={allTags}
          onSubmit={submitForm}
          onOpenAdmin={() => setTagsOpen(true)}
          onClose={() => setModal({ kind: "none" })}
        />
      )}
      {tagsOpen && (
        <AdminModal
          tags={allTags}
          categories={allCategories}
          onChanged={refresh}
          onClose={() => setTagsOpen(false)}
        />
      )}
      {modal.kind === "scan" && (
        <ScanModal
          defaultRoots={(() => {
            try {
              const roots = JSON.parse(settings.scan_roots ?? "[]");
              if (Array.isArray(roots) && roots.length > 0) return roots;
            } catch {
              /* fall through */
            }
            return settings.scan_root ? [settings.scan_root] : [];
          })()}
          onDone={() => {
            setModal({ kind: "none" });
            refresh();
            getSettings().then(setSettings).catch(() => {});
          }}
          onClose={() => setModal({ kind: "none" })}
        />
      )}
      {modal.kind === "settings" && (
        <SettingsModal
          settings={settings}
          onSaved={() => {
            setModal({ kind: "none" });
            getSettings()
              .then(setSettings)
              .catch((e) => setError(String(e)));
          }}
          onClose={() => setModal({ kind: "none" })}
        />
      )}
    </div>
  );
}
