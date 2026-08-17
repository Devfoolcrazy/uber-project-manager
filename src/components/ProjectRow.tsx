import type { Project, ProjectInsights } from "../types";
import { openInFinder, openInIde, openInTerminal } from "../api";
import {
  categoryColor,
  formatBytes,
  formatDate,
  shortenPath,
  statusLabel,
} from "../utils";

interface Props {
  project: Project;
  insight: ProjectInsights | null | undefined;
  selected: boolean;
  cursor: boolean;
  onSelect: () => void;
  onRelocate: () => void;
  onError: (msg: string) => void;
}

export default function ProjectRow({
  project,
  insight,
  selected,
  cursor,
  onSelect,
  onRelocate,
  onError,
}: Props) {
  const launch = (fn: (path: string) => Promise<void>) => (e: React.MouseEvent) => {
    e.stopPropagation();
    fn(project.path).catch((err) => onError(String(err)));
  };

  return (
    <article
      id={`project-${project.id}`}
      className={`project-row ${selected ? "selected" : ""} ${cursor ? "cursor" : ""}`}
      onClick={onSelect}
    >
      <span
        className="project-spine"
        style={{ background: categoryColor(project.category) }}
      />
      <div className="project-main">
        <div className="project-headline">
          <h3 className="project-name">{project.name}</h3>
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
          {insight?.techs.map((t) => (
            <span key={t} className="tech-badge">
              {t}
            </span>
          ))}
          {project.tags.map((t) => (
            <span key={t} className="project-tag">
              {t}
            </span>
          ))}
        </div>
        {project.description && (
          <p className="project-desc">{project.description}</p>
        )}
        <p className="project-path">
          {shortenPath(project.path)}
          {!project.path_exists && (
            <span className="path-missing" title="Ce dossier n'existe plus">
              introuvable
            </span>
          )}
        </p>
      </div>
      <div className="project-meta">
        <span className="project-date">{formatDate(project.updated_at)}</span>
        {insight?.git && (
          <span
            className="project-git"
            title={
              insight.git.dirty
                ? "Modifs non commitées"
                : "Rien à commiter"
            }
          >
            {insight.git.dirty && <span className="git-dirty">●</span>}
            {insight.git.branch}
          </span>
        )}
        {insight && (
          <span
            className="project-size"
            title={
              insight.reclaim_bytes > 0
                ? `dont ${formatBytes(insight.reclaim_bytes)} récupérables`
                : undefined
            }
          >
            {formatBytes(insight.size_bytes)}
          </span>
        )}
        {project.changelog_count > 0 && (
          <span className="project-log-count">
            {project.changelog_count} note{project.changelog_count > 1 ? "s" : ""}
          </span>
        )}
      </div>
      <div className="project-actions" onClick={(e) => e.stopPropagation()}>
        {!project.path_exists && (
          <button
            className="action-btn relocate-btn"
            title="Re-pointer vers le dossier déplacé"
            onClick={onRelocate}
          >
            Retrouver…
          </button>
        )}
        <button
          className="action-btn"
          title="Ouvrir dans le Finder"
          disabled={!project.path_exists}
          onClick={launch(openInFinder)}
        >
          Finder
        </button>
        <button
          className="action-btn"
          title="Ouvrir dans l'IDE"
          disabled={!project.path_exists}
          onClick={launch(openInIde)}
        >
          IDE
        </button>
        <button
          className="action-btn"
          title="Ouvrir un terminal ici"
          disabled={!project.path_exists}
          onClick={launch(openInTerminal)}
        >
          Term
        </button>
      </div>
    </article>
  );
}
