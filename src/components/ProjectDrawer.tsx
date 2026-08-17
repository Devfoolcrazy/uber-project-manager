import { useEffect, useState } from "react";
import type { ChangelogEntry, Project, ProjectInsights } from "../types";
import {
  addChangelogEntry,
  deleteChangelogEntry,
  deleteProject,
  listChangelog,
  openRepo,
} from "../api";
import {
  categoryColor,
  formatBytes,
  formatDateTime,
  shortenPath,
  statusLabel,
} from "../utils";

interface Props {
  project: Project;
  insight: ProjectInsights | null | undefined;
  onRefreshInsight: () => void;
  onClose: () => void;
  onEdit: () => void;
  onRelocate: () => void;
  onChanged: () => void;
  onDeleted: () => void;
  onError: (msg: string) => void;
}

export default function ProjectDrawer({
  project,
  insight,
  onRefreshInsight,
  onClose,
  onEdit,
  onRelocate,
  onChanged,
  onDeleted,
  onError,
}: Props) {
  const [entries, setEntries] = useState<ChangelogEntry[]>([]);
  const [draft, setDraft] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);

  const refresh = () =>
    listChangelog(project.id)
      .then(setEntries)
      .catch((e) => onError(String(e)));

  useEffect(() => {
    setDraft("");
    setConfirmDelete(false);
    refresh();
  }, [project.id]);

  const submitEntry = async () => {
    const text = draft.trim();
    if (!text) return;
    try {
      await addChangelogEntry(project.id, text);
      setDraft("");
      await refresh();
      onChanged();
    } catch (e) {
      onError(String(e));
    }
  };

  const removeEntry = async (id: number) => {
    try {
      await deleteChangelogEntry(id);
      await refresh();
      onChanged();
    } catch (e) {
      onError(String(e));
    }
  };

  const removeProject = async () => {
    try {
      await deleteProject(project.id);
      onDeleted();
    } catch (e) {
      onError(String(e));
    }
  };

  return (
    <section className="drawer">
      <header className="drawer-header">
        <span
          className="drawer-spine"
          style={{ background: categoryColor(project.category) }}
        />
        <div className="drawer-heading">
          <h2 className="drawer-title">{project.name}</h2>
          {project.category && (
            <span
              className="project-cat"
              style={{ color: categoryColor(project.category) }}
            >
              {project.category}
            </span>
          )}
          {project.status !== "active" && (
            <span className={`status-badge ${project.status}`}>
              {statusLabel(project.status)}
            </span>
          )}
        </div>
        <button className="ghost-btn" onClick={onClose} title="Fermer">
          ✕
        </button>
      </header>

      {project.description && (
        <p className="drawer-desc">{project.description}</p>
      )}

      {project.tags.length > 0 && (
        <div className="drawer-tags">
          {project.tags.map((t) => (
            <span key={t} className="project-tag">
              {t}
            </span>
          ))}
        </div>
      )}

      <p className="project-path drawer-path">
        {shortenPath(project.path)}
        {!project.path_exists && (
          <span className="path-missing" title="Ce dossier n'existe plus">
            introuvable
          </span>
        )}
      </p>
      {!project.path_exists && (
        <button className="secondary-btn" onClick={onRelocate}>
          Retrouver le dossier…
        </button>
      )}
      {project.repo && (
        <div className="drawer-repo">
          <span className="project-path drawer-path">
            {shortenPath(project.repo)}
          </span>
          <button
            className="secondary-btn"
            onClick={() => openRepo(project.repo).catch((e) => onError(String(e)))}
          >
            Ouvrir le repo
          </button>
        </div>
      )}
      <p className="drawer-dates">
        Créé le {formatDateTime(project.created_at)} · modifié le{" "}
        {formatDateTime(project.updated_at)}
      </p>

      <div className="drawer-toolbar">
        <button className="secondary-btn" onClick={onEdit}>
          Modifier
        </button>
        {confirmDelete ? (
          <>
            <button className="danger-btn" onClick={removeProject}>
              Confirmer la suppression
            </button>
            <button
              className="secondary-btn"
              onClick={() => setConfirmDelete(false)}
            >
              Annuler
            </button>
          </>
        ) : (
          <button className="danger-ghost-btn" onClick={() => setConfirmDelete(true)}>
            Supprimer
          </button>
        )}
      </div>

      {project.path_exists && (
        <div className="insights">
          <div className="insights-header">
            <h3 className="changelog-title">Repo &amp; disque</h3>
            {insight !== undefined && (
              <button className="ghost-btn" onClick={onRefreshInsight}>
                Actualiser
              </button>
            )}
          </div>
          {insight === undefined ? (
            <p className="modal-hint">Analyse du dossier…</p>
          ) : insight === null ? (
            <p className="modal-hint">Analyse impossible pour ce dossier.</p>
          ) : (
            <>
              {insight.techs.length > 0 && (
                <div className="drawer-tags">
                  {insight.techs.map((t) => (
                    <span key={t} className="tech-badge">
                      {t}
                    </span>
                  ))}
                </div>
              )}
              {insight.git ? (
                <p className="insight-line">
                  Branche <strong>{insight.git.branch}</strong>
                  {insight.git.last_commit_at && (
                    <>
                      {" "}
                      · dernier commit le{" "}
                      {formatDateTime(insight.git.last_commit_at)}
                    </>
                  )}
                  {insight.git.dirty ? (
                    <span className="git-dirty-label">
                      {" "}
                      · modifs non commitées
                    </span>
                  ) : (
                    " · rien à commiter"
                  )}
                </p>
              ) : (
                <p className="insight-line muted">Pas de repo git.</p>
              )}
              <p className="insight-line">
                <strong>{formatBytes(insight.size_bytes)}</strong> sur disque
                {insight.reclaim_bytes > 0 && (
                  <>
                    {" "}
                    · dont {formatBytes(insight.reclaim_bytes)} récupérables
                  </>
                )}
              </p>
              {insight.artifacts.slice(0, 6).map((a) => (
                <p key={a.rel_path} className="artifact-line">
                  {a.rel_path} — {formatBytes(a.bytes)}
                </p>
              ))}
            </>
          )}
        </div>
      )}

      <div className="changelog">
        <h3 className="changelog-title">Changelog</h3>
        <div className="changelog-form">
          <textarea
            className="changelog-input"
            placeholder="Nouvelle note… (ex. « ajout de l'auth, reste le refresh token »)"
            value={draft}
            rows={2}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) submitEntry();
            }}
          />
          <button
            className="primary-btn"
            onClick={submitEntry}
            disabled={!draft.trim()}
          >
            Ajouter la note
          </button>
        </div>
        {entries.length === 0 ? (
          <p className="changelog-empty">
            Aucune note pour l'instant. Chaque note est datée automatiquement.
          </p>
        ) : (
          <ol className="changelog-list">
            {entries.map((entry) => (
              <li key={entry.id} className="changelog-entry">
                <span className="changelog-date">
                  {formatDateTime(entry.created_at)}
                </span>
                <p className="changelog-text">{entry.entry}</p>
                <button
                  className="ghost-btn changelog-delete"
                  title="Supprimer la note"
                  onClick={() => removeEntry(entry.id)}
                >
                  ✕
                </button>
              </li>
            ))}
          </ol>
        )}
      </div>
    </section>
  );
}
